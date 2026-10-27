// Cámara del pizarrón fuera de React: desplazar y hacer zoom actualiza el
// DOM directamente (una transformación por cuadro) sin volver a renderizar
// componentes. React solo recibe instantáneas espaciadas para la culling y
// el nivel de detalle de las imágenes.

import { useEffect, useState } from "react";

import { throttle } from "@/utils/timing";

import { lerpCamera, type Camera } from "./geometry";

type Listener = (c: Camera) => void;

class CameraController {
  current: Camera = { x: 0, y: 0, zoom: 1 };
  viewport = { width: 1, height: 1 };
  private listeners = new Set<Listener>();
  private anim = 0;

  set(c: Camera, keepAnimation = false) {
    if (!keepAnimation) this.stopAnimation();
    if (!Number.isFinite(c.x) || !Number.isFinite(c.y) || !Number.isFinite(c.zoom)) return;
    this.current = c;
    for (const l of this.listeners) l(c);
  }

  subscribe(l: Listener): () => void {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  }

  stopAnimation() {
    if (this.anim) cancelAnimationFrame(this.anim);
    this.anim = 0;
  }

  /** Transición corta con desaceleración; instantánea si se reducen animaciones. */
  animateTo(target: Camera, ms = 220) {
    this.stopAnimation();
    const reduce = document.documentElement.dataset.reduceMotion === "true" ||
      matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduce || ms <= 0) {
      this.set(target);
      return;
    }
    const from = this.current;
    const t0 = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - t0) / ms);
      const eased = 1 - Math.pow(1 - t, 3);
      this.set(lerpCamera(from, target, eased), true);
      this.anim = t < 1 ? requestAnimationFrame(step) : 0;
    };
    this.anim = requestAnimationFrame(step);
  }
}

export const camera = new CameraController();

/** Instantánea de la cámara para React, como máximo cada `ms`. */
export function useCameraSnapshot(ms = 120): Camera {
  const [snap, setSnap] = useState(camera.current);
  useEffect(() => {
    const update = throttle((c: Camera) => setSnap(c), ms);
    return camera.subscribe(update);
  }, [ms]);
  return snap;
}
