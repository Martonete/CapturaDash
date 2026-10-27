# Pruebas de CanvasClip

Este documento separa con claridad tres cosas: lo que se **probó automáticamente**, lo que se **verificó a mano en la app real** (en Linux, bajo Xvfb) y lo que **todavía hay que probar en Windows real**, porque el entorno de desarrollo no era Windows.

## 1. Pruebas automatizadas

### Núcleo en Rust — `cd src-tauri && cargo test -p canvasclip-core` (24 pruebas)

| Área | Qué verifica |
|---|---|
| Migraciones | Se aplican una sola vez; reabrir la base no las repite; `quick_check` correcto. |
| Ingesta | Guarda original y miniatura, registra dimensiones. |
| Duplicados | La misma imagen otra vez → no se duplica. Los mismos píxeles llegando como DIB en lugar de PNG → mismo hash → no se duplica. |
| Imágenes | DIB con máscaras BI_BITFIELDS, DIB de 32 bits con alfa en cero (se vuelve opaco), miniaturas que conservan proporción, bytes PNG originales conservados tal cual, datos corruptos rechazados. |
| Retención | Límite alcanzado: se borran las más viejas no protegidas, con sus archivos. Favoritas y capturas usadas en pizarrones se conservan. Con más protegidas que el límite se conservan todas. |
| Borrado | Borrar una captura en uso exige confirmación (`InUse`); la base también lo impide con `RESTRICT`. Con confirmación se quita de los pizarrones y del disco. Quitar una imagen del pizarrón no la borra del historial. |
| Reutilización | Una captura en dos pizarrones = un solo archivo. |
| Pizarrones | Crear, renombrar (validación), duplicar (con grupos remapeados), eliminar (siempre queda uno), cámara, miniaturas de vista previa. |
| Guardado | Objetos cuya captura ya no existe se descartan sin hacer fallar la transacción. |
| Recuperación | Tras un "cierre inesperado" simulado: `.tmp` a medio escribir, archivo huérfano, miniatura faltante (se regenera), originales perdidos (la fila no usada se elimina, la usada se conserva). Una segunda pasada no hace nada. |
| Persistencia | Los datos sobreviven a descartar la conexión sin cierre ordenado. |
| Traslado de datos | Copia verificada a otra carpeta; no permite pisar un destino con datos. |
| Ajustes | Normalización de valores inválidos; JSON viejo sin campos nuevos se completa con valores por defecto. |
| Uso | Conteo de pizarrones por captura y capturas de objetos borrados. |

### App en Rust — `cargo test -p canvasclip` (4 pruebas)

- Zona del borde derecho: borde real vs. otro monitor a la derecha, esquinas excluidas, monitores con coordenadas negativas.
- Espera en el borde: requiere el tiempo configurado, no vuelve a disparar sin salir de la zona, se cancela con un botón presionado o al deslizar rápido por el borde.
- Validación del atajo: rechaza combinaciones con la tecla Windows, `Ctrl+C`, `Alt+Tab` y teclas sin modificador.

### Frontend — `npm test` (21 pruebas, Vitest)

- Cámara: conversión pantalla↔mundo, zoom que mantiene el punto bajo el cursor, límites, área visible, encuadre.
- Rectángulos: intersección, límites, redimensionar con y sin proporción (esquina opuesta fija, tamaño mínimo), mapeo entre marcos.
- Documento: agrupar/desagrupar, fusión de grupos, disolución de grupos de un miembro, orden z, color de notas, operaciones sin efecto, *diff* mínimo para guardar.
- Deshacer/rehacer: recorrido, descarte del futuro, límite de 200 pasos.

### Compilación para Windows

`cargo check --target x86_64-pc-windows-gnu` compila sin errores ni advertencias todo el código `cfg(windows)` (Win32: portapapeles, borde, ventanas, DWM). El workflow `.github/workflows/windows-build.yml` corre todas las pruebas **en Windows** (`windows-latest`, MSVC) y genera el instalador NSIS.

## 2. Verificado a mano en la app real (Linux + Xvfb)

La app completa (backend Rust + WebKitGTK) se ejecutó bajo Xvfb con capturas cargadas mediante el mismo pipeline de ingesta (`cargo run -p canvasclip-core --example import`):

- Arranque: abre la base, reconcilia, registra el atajo global, crea el pizarrón predeterminado.
- Las miniaturas llegan por el protocolo `ccimg`.
- Botón **+**: Rust inserta en SQLite, la ventana principal ubica la imagen en el centro de la vista, la selecciona y el indicador "En 1 pizarrón" se actualiza.
- **Atajo global** `Alt+Shift+V`: abre el panel en el borde derecho del área de trabajo; un segundo toque lo cierra; `Esc` lo cierra; reabrir durante la animación de cierre lo deja abierto (se corrigió una carrera).
- **Arrastre entre ventanas** del panel al pizarrón: la imagen se agrega y se guarda.
- **Persistencia**: cerrar y reabrir la app restaura el pizarrón tal como estaba.
- Quitar una imagen del pizarrón: la captura sigue en el historial y su indicador de uso se actualiza.

Problemas encontrados y corregidos en estas pruebas: cierre espurio del panel por un rebote de foco al abrir, la orden tardía de ocultar después de reabrir, culling calculado con un viewport desactualizado, origen del mundo en la esquina en pizarrones nuevos, imágenes agregadas seguidas que quedaban apiladas y el aviso tapando los controles.

### Recorrido visual en Chromium (motor equivalente a WebView2)

`tests/e2e/` contiene un *harness* que carga la interfaz real con el IPC simulado por la API oficial `@tauri-apps/api/mocks` (solo para QA; no se incluye en la app). Con Playwright se verificó: arrastre desde el cajón, nota con doble clic y escritura, agrupar (`Ctrl+G`), mover el grupo completo, deshacer/rehacer del movimiento, aislar un miembro con doble clic, redimensionar una nota (libre) y una imagen (proporción 1,600 → 1,600), borrar y deshacer el borrado, `Ctrl` + rueda, encuadrar con `F`, menú contextual, biblioteca, ajustes y panel. Sin errores de consola.

Para repetirlo: `npm run dev` y abrir `http://localhost:1420/tests/e2e/main.html` (las imágenes se sirven desde Playwright; en un navegador común se ven vacías).

## 3. Pendiente de probar en Windows real

No se pudo probar porque el entorno de desarrollo era Linux (y la red bloqueaba el SDK de Windows). **El código existe y compila para Windows, pero falta validarlo en el sistema:**

| Prueba | Qué observar |
|---|---|
| `Win+Shift+S` (Recortes) y `Impr Pant` | Aparece el aviso y la captura en el panel, una sola vez. |
| Capturas consecutivas rápidas | Ninguna se pierde ni se duplica. |
| Captura 4K / multimonitor | Se guarda completa; el panel no se traba. |
| Copiar imagen desde Chrome/Edge/Firefox | Se registra. |
| Copiar texto o celdas en Word/Excel | **No** se registra. |
| Gestor de contraseñas | Su contenido no se registra. |
| Copiar una captura desde CanvasClip | Se puede pegar en Word, Paint, Discord, navegador; **no** aparece duplicada. |
| Portapapeles ocupado | Abrir una app que lo retenga y capturar: se reintenta. |
| Borde derecho | Abre tras el tiempo configurado; no abre en las esquinas ni con un juego/video en pantalla completa; con dos monitores solo en el borde exterior. |
| Foco | Desde el borde no roba el foco (se puede seguir escribiendo); con el atajo sí, y `Esc` lo cierra. |
| Arrastre panel → pizarrón | Funciona entre las dos ventanas WebView2. |
| Escalado 100 %, 125 %, 150 % | Panel y aviso con el tamaño correcto; imágenes nítidas en el pizarrón; mover el panel entre monitores con distinta escala. |
| Acrylic / esquinas redondeadas | Windows 11 y Windows 10. |
| Inicio con Windows | Arranca minimizado en la bandeja. |
| Reiniciar Windows | Historial y pizarrones intactos. |
| Cierre forzado (Administrador de tareas) | Al reabrir no hay pérdida ni corrupción. |
| Disco lleno | Aviso "No hay espacio suficiente"; sin datos corruptos. |
| Sin permisos en la carpeta de datos | Mensaje claro al iniciar. |
| Instalador | Instala por usuario, crea acceso directo, desinstala limpio. |

### Rendimiento

No se midió en hardware real. Objetivos de diseño: interacción del pizarrón a 60 FPS (la cámara no re-renderiza React; culling de objetos fuera de vista; miniaturas de máx. 640 px), decodificación de imágenes fuera del hilo de la interfaz (hilo de ingesta dedicado) y memoria estable con cientos de capturas. **Hay que medirlo** (Administrador de tareas: CPU en reposo ≈ 0 %, memoria con 500 capturas y un pizarrón con 200 imágenes).
