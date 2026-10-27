// URLs del protocolo `ccimg` (src-tauri/src/protocol.rs). En Windows los
// protocolos propios se sirven como http://<esquema>.localhost.

const isWindows = typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent);
const base = isWindows ? "http://ccimg.localhost" : "ccimg://localhost";

export function thumbUrl(id: string): string {
  return `${base}/thumb/${id}`;
}

export function originalUrl(id: string): string {
  return `${base}/orig/${id}`;
}

/** Tipo de dato de arrastre para mover capturas entre ventanas. */
export const CAPTURE_DRAG_TYPE = "application/x-canvasclip-capture";
