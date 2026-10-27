// Geometría del pizarrón. Convención de cámara:
//   pantalla = mundo * zoom + (x, y)

export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

export const MIN_ZOOM = 0.05;
export const MAX_ZOOM = 8;

export const clampZoom = (z: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));

export function screenToWorld(c: Camera, sx: number, sy: number): Point {
  return { x: (sx - c.x) / c.zoom, y: (sy - c.y) / c.zoom };
}

export function worldToScreen(c: Camera, wx: number, wy: number): Point {
  return { x: wx * c.zoom + c.x, y: wy * c.zoom + c.y };
}

/** Zoom manteniendo fijo el punto de pantalla (sx, sy) — "zoom hacia el cursor". */
export function zoomAt(c: Camera, nextZoom: number, sx: number, sy: number): Camera {
  const zoom = clampZoom(nextZoom);
  const w = screenToWorld(c, sx, sy);
  return { zoom, x: sx - w.x * zoom, y: sy - w.y * zoom };
}

/** Rectángulo del mundo visible en un viewport de (w, h) píxeles. */
export function visibleWorld(c: Camera, w: number, h: number, margin = 0): Rect {
  const tl = screenToWorld(c, 0, 0);
  const mw = (w / c.zoom) * margin;
  const mh = (h / c.zoom) * margin;
  return { x: tl.x - mw, y: tl.y - mh, width: w / c.zoom + 2 * mw, height: h / c.zoom + 2 * mh };
}

export function intersects(a: Rect, b: Rect): boolean {
  return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
}

export function containsRect(outer: Rect, inner: Rect): boolean {
  return (
    inner.x >= outer.x &&
    inner.y >= outer.y &&
    inner.x + inner.width <= outer.x + outer.width &&
    inner.y + inner.height <= outer.y + outer.height
  );
}

export function rectFromPoints(a: Point, b: Point): Rect {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y) };
}

export function boundsOf(rects: Rect[]): Rect | null {
  if (rects.length === 0) return null;
  let x1 = Infinity;
  let y1 = Infinity;
  let x2 = -Infinity;
  let y2 = -Infinity;
  for (const r of rects) {
    x1 = Math.min(x1, r.x);
    y1 = Math.min(y1, r.y);
    x2 = Math.max(x2, r.x + r.width);
    y2 = Math.max(y2, r.y + r.height);
  }
  return { x: x1, y: y1, width: x2 - x1, height: y2 - y1 };
}

/** Cámara que encuadra `r` en un viewport (w, h) con margen en píxeles. */
export function fitCamera(r: Rect, w: number, h: number, padding = 64, maxZoom = 1): Camera {
  const zoom = clampZoom(
    Math.min((w - 2 * padding) / Math.max(r.width, 1), (h - 2 * padding) / Math.max(r.height, 1), maxZoom),
  );
  return {
    zoom,
    x: w / 2 - (r.x + r.width / 2) * zoom,
    y: h / 2 - (r.y + r.height / 2) * zoom,
  };
}

export type Handle = "nw" | "ne" | "sw" | "se";

/**
 * Nuevo rectángulo al arrastrar una esquina `handle` por (dx, dy) en unidades
 * del mundo. Con `keepAspect` conserva la proporción (imágenes, selecciones
 * múltiples). La esquina opuesta queda fija.
 */
export function resizeRect(start: Rect, handle: Handle, dx: number, dy: number, keepAspect: boolean, min = 16): Rect {
  const left = handle === "nw" || handle === "sw";
  const top = handle === "nw" || handle === "ne";
  let width = start.width + (left ? -dx : dx);
  let height = start.height + (top ? -dy : dy);

  if (keepAspect) {
    const ratio = start.width / start.height;
    // Gana el eje que más se movió en proporción.
    const scale = Math.max(width / start.width, height / start.height);
    width = start.width * scale;
    height = width / ratio;
    const minScale = Math.max(min / start.width, min / start.height);
    if (scale < minScale) {
      width = start.width * minScale;
      height = width / ratio;
    }
  } else {
    width = Math.max(min, width);
    height = Math.max(min, height);
  }

  const x = left ? start.x + start.width - width : start.x;
  const y = top ? start.y + start.height - height : start.y;
  return { x, y, width, height };
}

/** Lleva un rectángulo de un marco `from` a otro `to` (escala uniforme o no). */
export function mapRect(r: Rect, from: Rect, to: Rect): Rect {
  const sx = to.width / from.width;
  const sy = to.height / from.height;
  return {
    x: to.x + (r.x - from.x) * sx,
    y: to.y + (r.y - from.y) * sy,
    width: r.width * sx,
    height: r.height * sy,
  };
}

/** Interpolación de cámara para transiciones (encuadrar, zoom con botones). */
export function lerpCamera(a: Camera, b: Camera, t: number): Camera {
  // El zoom se interpola en escala logarítmica para que se perciba uniforme.
  const zoom = Math.exp(Math.log(a.zoom) + (Math.log(b.zoom) - Math.log(a.zoom)) * t);
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, zoom };
}
