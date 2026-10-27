# CanvasClip

Historial visual de capturas de pantalla para Windows 10 y 11, con panel lateral desplegable y pizarrón infinito.

1. Hacés una captura con `Win + Shift + S` (o copiás cualquier imagen).
2. CanvasClip la guarda sola, con su miniatura, sin abrir ventanas. La imagen sigue en el portapapeles como siempre.
3. Abrís el historial con **Alt + Shift + V** (configurable) o apoyando el cursor en el **borde derecho** de la pantalla.
4. Volvés a copiar cualquier captura con doble clic, o la llevás a un pizarrón para organizarla con notas y grupos.

Todo se guarda automáticamente y se conserva al reiniciar Windows.

## Uso rápido

| Acción | Cómo |
|---|---|
| Abrir/cerrar historial | `Alt+Shift+V` · cursor en el borde derecho · clic en el aviso |
| Copiar una captura | Doble clic · `Enter` (con el panel abierto por atajo) |
| Llevar al pizarrón | Botón **+** · arrastrar al pizarrón · menú **…** → *Agregar a otro pizarrón* |
| Favorita / eliminar | Menú **…** de la tarjeta (las favoritas nunca se borran solas) |
| Pausar captura automática | Menú del ícono en la bandeja · Ajustes |

**Pizarrón**

| Acción | Atajo |
|---|---|
| Desplazar | Rueda (`Shift` + rueda: horizontal) · botón central · `Espacio` + arrastrar |
| Zoom hacia el cursor | `Ctrl` + rueda · pellizco en el touchpad · `Ctrl +` / `Ctrl −` / `Ctrl 0` |
| Encuadrar todo | `F` |
| Seleccionar | Clic · `Shift` + clic · arrastrar en el fondo · `Ctrl+A` |
| Mover / redimensionar | Arrastrar · esquinas (las imágenes conservan su proporción) |
| Copiar / pegar | `Ctrl+C` · `Ctrl+V` (imagen → objeto, texto → nota) |
| Agrupar / desagrupar | `Ctrl+G` · `Ctrl+Shift+G` · doble clic en un grupo para editar un elemento |
| Nota | Doble clic en el fondo · botón de nota abajo; doble clic o `Enter` para editar; `Esc` para terminar |
| Quitar del pizarrón | `Supr` (la captura queda en el historial) |
| Deshacer / rehacer | `Ctrl+Z` · `Ctrl+Shift+Z` / `Ctrl+Y` |

También se pueden soltar archivos PNG/JPG desde el Explorador.

## Instalar

**Opción A — instalador ya compilado (recomendado).** Cada push a la rama ejecuta el workflow *Windows build* de GitHub Actions, que corre las pruebas en Windows y genera `CanvasClip_0.1.0_x64-setup.exe`. Para descargarlo: pestaña **Actions** del repositorio → la última ejecución de *Windows build* → **Artifacts** → `CanvasClip-windows-installer`. El instalador es por usuario (no pide administrador) y, si hace falta, instala WebView2.

> El instalador no está firmado digitalmente: Windows SmartScreen puede advertirlo la primera vez (*Más información → Ejecutar de todas formas*).

**Opción B — compilar en tu PC.** Ver abajo.

## Compilar en Windows

Requisitos (una sola vez):

1. **Microsoft C++ Build Tools** con la carga de trabajo *Desarrollo para el escritorio con C++* (incluye el SDK de Windows): <https://visualstudio.microsoft.com/visual-cpp-build-tools/>
2. **Rust** (estable): <https://rustup.rs>
3. **Node.js 22 LTS**: <https://nodejs.org>
4. **Git**: <https://git-scm.com>
5. WebView2 ya viene con Windows 10 (actualizado) y Windows 11.

En PowerShell:

```powershell
cd C:\Users\marti\Documents
git clone -b claude/festive-hypatia-5s3ro0 https://github.com/Martonete/CapturaDash.git Docudash
cd Docudash
npm ci
```

Desarrollo (recarga en caliente de la interfaz):

```powershell
npm run app:dev
```

Instalador de producción:

```powershell
npm run app:build
```

El instalador queda en `src-tauri\target\release\bundle\nsis\`.

Pruebas:

```powershell
npm test                              # frontend (Vitest)
npm run build                         # necesario una vez antes de las pruebas de Rust
cd src-tauri; cargo test --workspace  # núcleo + app
```

## Datos

- Ubicación predeterminada: `%LOCALAPPDATA%\com.canvasclip.app\` (base SQLite, `originals\`, `thumbs\`). Se puede cambiar en Ajustes → Capturas → Almacenamiento: se copian los datos, se verifica la copia y la app se reinicia; la carpeta anterior no se borra.
- Registro de diagnóstico: `%LOCALAPPDATA%\com.canvasclip.app\logs\`.
- Límite del historial: 500 capturas por defecto. Las **favoritas** y las **usadas en algún pizarrón** no cuentan para el límite y nunca se borran automáticamente.
- Nada sale de tu PC: no hay red, cuentas ni telemetría.

## Documentación

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): revisión técnica, decisiones (motor del pizarrón, almacenamiento, integración con Windows), modelo de datos, licencias.
- [`docs/TESTING.md`](docs/TESTING.md): qué se probó, cómo, resultados y qué falta probar en Windows real.
