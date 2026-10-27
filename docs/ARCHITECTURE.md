# Arquitectura de CanvasClip

## Revisión técnica de la especificación

Riesgos reales encontrados y cómo se resolvieron:

| Tema | Riesgo | Decisión |
|---|---|---|
| Detección de capturas | Sondear el portapapeles desde la interfaz es costoso e impreciso. | Ventana Win32 de solo mensajes + `AddClipboardFormatListener` (evento `WM_CLIPBOARDUPDATE`). Sin sondeo. |
| Portapapeles bloqueado | Otra app puede tenerlo abierto (`OpenClipboard` falla). | 8 reintentos cortos con espera creciente y, si sigue ocupado, hasta 5 reintentos más cada 400 ms. |
| Duplicados al volver a copiar | Si CanvasClip copia una captura, el monitor la vería como nueva. | Doble protección: se ignora el número de secuencia del portapapeles generado por la propia escritura, y además se deduplica por hash del contenido (blake3 de los píxeles, así PNG y DIB de la misma imagen dan el mismo hash). |
| Contenido sensible | Los gestores de contraseñas marcan su contenido para que no se registre. | Se respetan los formatos `ExcludeClipboardContentFromMonitorProcessing` y `Clipboard Viewer Ignore`. |
| Activación por borde | Un hook global de mouse agrega latencia a todo el sistema y aperturas accidentales. | Hilo nativo que lee `GetCursorPos` cada 40 ms. Solo cuenta el borde real (sin monitor a la derecha), excluye las esquinas, exige quietud durante el tiempo configurado, no abre con botones presionados ni con apps a pantalla completa (`SHQueryUserNotificationState`) y requiere salir de la zona para volver a armarse. |
| Foco | Abrir el panel no debe interrumpir lo que se escribe. | Desde el borde se muestra con `SW_SHOWNOACTIVATE` (sin foco) y se cierra al alejar el cursor. Con el atajo sí toma el foco (hay intención de usar el teclado) y se cierra con Esc o al hacer clic afuera. |
| Arrastrar entre ventanas | El panel es una ventana aparte del pizarrón. | Arrastre HTML5 con un tipo propio (`application/x-canvasclip-capture`); `dragDropEnabled: false` para que WebView2 no lo intercepte. Además, el pizarrón tiene un cajón de historial integrado donde el arrastre es dentro de la misma ventana. |
| Licencias del motor de pizarrón | tldraw requiere licencia comercial en producción. | Motor propio (ver abajo). |
| Pérdida de datos | Cierres inesperados, cortes de luz, disco lleno. | SQLite en WAL con `synchronous=FULL`, archivos escritos de forma atómica (temporal → `fsync` → renombrar), fila insertada solo después de los archivos y reconciliación al arrancar. |
| Contradicción menor | "Eliminar del historial" vs "imágenes usadas en pizarrones no se eliminan". | Las capturas usadas nunca se borran *automáticamente*. El borrado manual de una captura en uso pide confirmación explícita y la quita también de los pizarrones. |

## Motor del pizarrón: implementación propia

| Criterio | tldraw | Excalidraw | Konva | **Propio (React + transform 2D)** |
|---|---|---|---|---|
| Licencia | Comercial en producción ⚠️ | MIT | MIT | — |
| Integración | Trae su propia interfaz completa | Estética propia, difícil de quitar | Canvas: notas editables requieren superponer DOM | Total |
| Rendimiento | Alto | Medio | Alto | Alto con culling y LOD |
| Persistencia | Formato propio | Formato propio | Manual | Modelo de datos propio = SQLite directo |
| Tamaño | Grande | Grande | Medio | ≈ 60 KB del bundle principal |

Cómo funciona:

- Un solo elemento "mundo" con `translate3d(...) scale(...)`. La cámara vive **fuera de React** (`features/canvas/camera.ts`): desplazar o hacer zoom solo modifica el estilo en un `requestAnimationFrame`, sin re-renderizar componentes.
- Durante el movimiento se activa `will-change: transform`; al detenerse se quita para que el navegador vuelva a rasterizar con nitidez.
- **Culling:** se renderizan solo los objetos que intersectan la vista (+50 % de margen) y los seleccionados.
- **Nivel de detalle:** cada imagen muestra su miniatura (máx. 640 px); el original se carga encima solo cuando el tamaño en pantalla lo justifica, con histéresis para no alternar. El original nunca se modifica: redimensionar cambia solo ancho y alto del objeto.
- **Deshacer/rehacer:** instantáneas de un documento inmutable con estructura compartida (200 pasos). Arrastrar o redimensionar es un único paso.
- **Guardado:** cada cambio programa un guardado a los 350 ms con un *diff* contra lo último confirmado; Rust lo aplica en **una transacción**. La cámara se guarda aparte.

## Procesos e hilos

```
Proceso CanvasClip (Rust)
├─ Hilo principal: bucle de eventos de Tauri, ventanas, bandeja
├─ canvasclip-clipboard: ventana de solo mensajes, WM_CLIPBOARDUPDATE
├─ canvasclip-ingest: decodificar → hash → guardar → miniatura → SQLite → retención
├─ canvasclip-edge: posición del cursor en el borde derecho
└─ Pool async: comandos de las ventanas (lecturas/escrituras de SQLite, protocolo ccimg)

WebView2 (una por ventana)
├─ main   (index.html)  pizarrón, biblioteca, ajustes
├─ panel  (panel.html)  historial flotante
└─ toast  (toast.html)  aviso "Captura guardada"
```

Las imágenes llegan a las ventanas por el protocolo propio `ccimg` (`http://ccimg.localhost/thumb/<id>` y `/orig/<id>` en Windows), servido por id, sin exponer rutas, y cacheado indefinidamente porque los ids nunca se reutilizan.

## Almacenamiento

```
%LOCALAPPDATA%\com.canvasclip.app\      (configurable en Ajustes)
├─ canvasclip.db (+ -wal, -shm)
├─ originals\<id>.png    original sin modificar (los PNG del portapapeles se guardan tal cual)
└─ thumbs\<id>.jpg|png   miniatura (JPEG si es opaca, PNG si tiene transparencia)
```

La ubicación elegida se guarda en `%APPDATA%\com.canvasclip.app\location.json`, fuera de la base (que vive dentro de esa carpeta). Cambiarla copia la base (`VACUUM INTO`) y los archivos, verifica la copia, reinicia la app y **no borra** la carpeta anterior.

## Modelo de datos

Ver `src-tauri/migrations/0001_init.sql`. Resumen:

- `captures`: una fila por imagen única (`hash` único), con rutas relativas, dimensiones, tamaño, favorita y fecha.
- `boards`: nombre, fechas y cámara (`camera_x`, `camera_y`, `zoom`).
- `board_objects`: imágenes y notas de cada pizarrón. Las imágenes referencian `captures` con `ON DELETE RESTRICT`: la base impide borrar una captura en uso por accidente. Las notas guardan texto y color en `props` (JSON).
- `board_groups`: grupos (un nivel; agrupar grupos los fusiona).
- `settings`: preferencias como un único JSON (agregar una preferencia nueva no requiere migración).

Las migraciones se versionan con `PRAGMA user_version` y cada una corre en su transacción.

### Retención

Después de cada captura nueva se conservan las `historyLimit` capturas **no protegidas** más recientes. Protegida = favorita **o** usada en al menos un pizarrón. Las protegidas no cuentan para el límite y nunca se borran solas, aunque superen el límite. Quitar una imagen de un pizarrón no la borra del historial.

### Reconciliación al arrancar

En segundo plano: borra `.tmp` de escrituras interrumpidas y archivos huérfanos (con más de 60 s de antigüedad), elimina filas sin original que nadie usa (las que se usan se conservan y el pizarrón muestra "Imagen no disponible"), y regenera miniaturas faltantes desde el original.

## Estructura

```
src/                         Frontend (React + TypeScript)
├─ app/                      Entradas de cada ventana, ventana principal, aviso
├─ components/
│  ├─ common/                Botones, menú contextual, interruptor, segmentado
│  ├─ clipboard/             Lista y tarjeta de capturas (panel y cajón)
│  ├─ sidebar/               Ventana del panel lateral
│  ├─ canvas/                Pizarrón, objetos, selección, barra contextual, controles
│  ├─ notes/                 Notas adhesivas
│  ├─ boards/                Selector/biblioteca de pizarrones
│  └─ settings/              Ajustes y grabador de atajos
├─ features/
│  ├─ canvas/                Geometría, documento, cámara, deshacer, guardado, interacción
│  └─ history/               Acciones sobre capturas
├─ services/                 IPC tipado, eventos, URLs de imágenes
├─ stores/                   Estado (zustand): ajustes, historial, pizarrones, UI
├─ styles/                   Variables de diseño y base
└─ types/                    Tipos compartidos con Rust

src-tauri/
├─ core/                     Crate sin Tauri ni Windows: base de datos, almacenamiento,
│                            imágenes, ingesta, retención, mantenimiento (+ pruebas)
├─ src/
│  ├─ clipboard/             Win32: listener, lectura y escritura
│  ├─ windows/               Panel, aviso, borde, utilidades Win32
│  ├─ shortcuts/             Atajo global
│  ├─ commands/              Comandos invocables desde las ventanas
│  ├─ engine.rs              Hilo de ingesta
│  ├─ protocol.rs            Protocolo ccimg
│  ├─ tray.rs, location.rs, state.rs, events.rs, lib.rs
├─ migrations/               SQL versionado
└─ capabilities/             Permisos de las ventanas
```

Diferencia con la estructura propuesta: `storage/` y `database/` viven en el crate `core` en lugar de `src-tauri/src/`. Así se compilan y prueban en cualquier sistema sin WebView ni Windows, y la capa de la app queda limitada a la integración con el sistema.

## Dependencias y licencias

Todas permisivas (MIT, Apache-2.0, ISC, BSD, Zlib u OFL para la fuente):

| Paquete | Uso | Licencia |
|---|---|---|
| tauri 2.12, plugins global-shortcut / autostart / single-instance / dialog / log | Aplicación de escritorio | MIT / Apache-2.0 |
| rusqlite 0.37 (SQLite incluido) | Base de datos | MIT (SQLite: dominio público) |
| image 0.25 (png, jpeg, bmp) | Decodificar, miniaturas | MIT / Apache-2.0 |
| blake3 | Hash de contenido | CC0 / Apache-2.0 |
| windows-sys 0.61 | Win32 | MIT / Apache-2.0 |
| uuid, serde, thiserror, parking_lot, crossbeam-channel, log | Utilidades | MIT / Apache-2.0 |
| react 19, react-dom | Interfaz | MIT |
| zustand 5 | Estado | MIT |
| lucide-react | Íconos | ISC |
| @fontsource-variable/inter | Tipografía Inter | OFL-1.1 |
