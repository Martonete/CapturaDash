import { describe, expect, it } from "vitest";

import {
  boundsOf,
  clampZoom,
  fitCamera,
  intersects,
  mapRect,
  MAX_ZOOM,
  MIN_ZOOM,
  resizeRect,
  screenToWorld,
  visibleWorld,
  worldToScreen,
  zoomAt,
} from "@/features/canvas/geometry";

describe("cámara", () => {
  it("convierte pantalla ↔ mundo de forma inversa", () => {
    const c = { x: 120, y: -40, zoom: 1.75 };
    const w = screenToWorld(c, 300, 200);
    const s = worldToScreen(c, w.x, w.y);
    expect(s.x).toBeCloseTo(300);
    expect(s.y).toBeCloseTo(200);
  });

  it("el zoom mantiene fijo el punto bajo el cursor", () => {
    const c = { x: 50, y: 80, zoom: 1 };
    const before = screenToWorld(c, 400, 300);
    const z = zoomAt(c, 2.5, 400, 300);
    const after = screenToWorld(z, 400, 300);
    expect(after.x).toBeCloseTo(before.x);
    expect(after.y).toBeCloseTo(before.y);
    expect(z.zoom).toBe(2.5);
  });

  it("limita el zoom", () => {
    expect(clampZoom(100)).toBe(MAX_ZOOM);
    expect(clampZoom(0)).toBe(MIN_ZOOM);
  });

  it("calcula el área visible del mundo", () => {
    const v = visibleWorld({ x: -100, y: -50, zoom: 2 }, 800, 600);
    expect(v).toEqual({ x: 50, y: 25, width: 400, height: 300 });
  });

  it("encuadra un rectángulo centrado sin pasar de 100 %", () => {
    const c = fitCamera({ x: 0, y: 0, width: 100, height: 100 }, 1000, 800, 50, 1);
    expect(c.zoom).toBe(1);
    const center = screenToWorld(c, 500, 400);
    expect(center.x).toBeCloseTo(50);
    expect(center.y).toBeCloseTo(50);
    const big = fitCamera({ x: 0, y: 0, width: 4000, height: 1000 }, 1000, 800, 50, 1);
    expect(big.zoom).toBeCloseTo(900 / 4000);
  });
});

describe("rectángulos", () => {
  it("intersección y límites", () => {
    expect(intersects({ x: 0, y: 0, width: 10, height: 10 }, { x: 9, y: 9, width: 5, height: 5 })).toBe(true);
    expect(intersects({ x: 0, y: 0, width: 10, height: 10 }, { x: 10, y: 0, width: 5, height: 5 })).toBe(false);
    expect(boundsOf([])).toBeNull();
    expect(
      boundsOf([
        { x: 0, y: 0, width: 10, height: 10 },
        { x: -5, y: 20, width: 5, height: 5 },
      ]),
    ).toEqual({ x: -5, y: 0, width: 15, height: 25 });
  });

  it("redimensionar conserva la proporción y la esquina opuesta", () => {
    const start = { x: 0, y: 0, width: 200, height: 100 };
    const r = resizeRect(start, "se", 100, 0, true);
    expect(r).toEqual({ x: 0, y: 0, width: 300, height: 150 });
    const nw = resizeRect(start, "nw", -100, -10, true);
    expect(nw.x + nw.width).toBeCloseTo(200);
    expect(nw.y + nw.height).toBeCloseTo(100);
    expect(nw.width / nw.height).toBeCloseTo(2);
  });

  it("redimensionar libre respeta el mínimo", () => {
    const r = resizeRect({ x: 0, y: 0, width: 100, height: 100 }, "se", -500, -500, false, 16);
    expect(r.width).toBe(16);
    expect(r.height).toBe(16);
    const keep = resizeRect({ x: 0, y: 0, width: 100, height: 50 }, "se", -500, -500, true, 16);
    expect(keep.height).toBeCloseTo(16);
    expect(keep.width).toBeCloseTo(32);
  });

  it("mapea rectángulos entre marcos", () => {
    const from = { x: 0, y: 0, width: 100, height: 100 };
    const to = { x: 100, y: 100, width: 200, height: 200 };
    expect(mapRect({ x: 50, y: 0, width: 50, height: 50 }, from, to)).toEqual({ x: 200, y: 100, width: 100, height: 100 });
  });
});
