// Interacción con el pizarrón: seleccionar, mover, redimensionar, desplazar,
// zoom, selección por recuadro, menú contextual y soltar capturas/archivos.

import { useCallback, useEffect, useRef, useState, type DragEvent, type MouseEvent, type PointerEvent, type RefObject } from "react";

import { api } from "@/services/api";
import { CAPTURE_DRAG_TYPE } from "@/services/images";
import type { BoardObject } from "@/types/models";

import { addCapture, addNote, importFiles, setLastPointer } from "./actions";
import { useBoard } from "./boardStore";
import { camera } from "./camera";
import { expandToGroups, rectOf, updateObjects } from "./document";
import {
  boundsOf,
  intersects,
  mapRect,
  rectFromPoints,
  resizeRect,
  screenToWorld,
  zoomAt,
  type Camera,
  type Handle,
  type Point,
  type Rect,
} from "./geometry";

const DRAG_THRESHOLD = 3;

type Gesture =
  | { kind: "pan"; sx: number; sy: number; cam: Camera }
  | { kind: "pending"; sx: number; sy: number; unit: string[]; shift: boolean; wasSelected: boolean }
  | { kind: "move"; sx: number; sy: number; ids: string[] }
  | { kind: "resize"; sx: number; sy: number; ids: string[]; box: Rect; handle: Handle; keepAspect: boolean }
  | { kind: "marquee"; start: Point; base: string[] };

export type CanvasMenuTarget = { kind: "object"; x: number; y: number } | { kind: "canvas"; x: number; y: number; world: Point };

export function useCanvasInput(viewportRef: RefObject<HTMLDivElement | null>) {
  const gesture = useRef<Gesture | null>(null);
  const space = useRef(false);
  const [marquee, setMarquee] = useState<Rect | null>(null);
  const [interacting, setInteracting] = useState(false);
  const [panMode, setPanMode] = useState<"none" | "ready" | "active">("none");
  const [menu, setMenu] = useState<CanvasMenuTarget | null>(null);
  const [dropActive, setDropActive] = useState(false);

  const local = useCallback(
    (clientX: number, clientY: number) => {
      const r = viewportRef.current!.getBoundingClientRect();
      return { sx: clientX - r.left, sy: clientY - r.top };
    },
    [viewportRef],
  );

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    if (target.closest("[data-ui]")) return;
    const { sx, sy } = local(e.clientX, e.clientY);
    const st = useBoard.getState();
    if (st.status !== "ready") return;

    // Desplazamiento libre: botón central o Espacio + arrastrar.
    if (e.button === 1 || (e.button === 0 && space.current)) {
      e.preventDefault();
      gesture.current = { kind: "pan", sx, sy, cam: camera.current };
      setPanMode("active");
      e.currentTarget.setPointerCapture(e.pointerId);
      return;
    }
    if (e.button !== 0) return;

    const handle = target.closest<HTMLElement>("[data-handle]")?.dataset.handle as Handle | undefined;
    if (handle && st.selection.length) {
      const objs = st.selection.map((id) => st.doc.objects[id]).filter(Boolean);
      const box = boundsOf(objs.map(rectOf));
      if (!box) return;
      st.beginGesture();
      gesture.current = {
        kind: "resize",
        sx,
        sy,
        ids: objs.map((o) => o.id),
        box,
        handle,
        keepAspect: !(objs.length === 1 && objs[0].kind === "note"),
      };
      setInteracting(true);
      e.currentTarget.setPointerCapture(e.pointerId);
      return;
    }

    const objEl = target.closest<HTMLElement>("[data-object-id]");
    if (objEl) {
      const id = objEl.dataset.objectId!;
      if (st.editingId === id) return; // el área de texto maneja el puntero
      if (st.editingId) st.setEditing(null);
      const o = st.doc.objects[id];
      if (!o) return;
      let isolated = st.isolatedGroup;
      if (isolated && o.groupId !== isolated) {
        isolated = null;
        st.setIsolated(null);
      }
      const unit = expandToGroups(st.doc, [id], isolated);
      const wasSelected = unit.every((u) => st.selection.includes(u));
      if (e.shiftKey) {
        st.select(wasSelected ? st.selection.filter((s) => !unit.includes(s)) : [...new Set([...st.selection, ...unit])]);
      } else if (!wasSelected) {
        st.select(unit);
      }
      gesture.current = { kind: "pending", sx, sy, unit, shift: e.shiftKey, wasSelected };
      e.currentTarget.setPointerCapture(e.pointerId);
      return;
    }

    // Espacio vacío: recuadro de selección.
    if (st.editingId) st.setEditing(null);
    st.setIsolated(null);
    const base = e.shiftKey ? st.selection : [];
    if (!e.shiftKey) st.select([]);
    gesture.current = { kind: "marquee", start: screenToWorld(camera.current, sx, sy), base };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const { sx, sy } = local(e.clientX, e.clientY);
    const c = camera.current;
    setLastPointer(screenToWorld(c, sx, sy));
    const g = gesture.current;
    if (!g) return;
    const st = useBoard.getState();

    switch (g.kind) {
      case "pan":
        camera.set({ zoom: g.cam.zoom, x: g.cam.x + (sx - g.sx), y: g.cam.y + (sy - g.sy) });
        return;
      case "pending": {
        if (Math.hypot(sx - g.sx, sy - g.sy) < DRAG_THRESHOLD) return;
        const ids = st.selection.filter((id) => st.doc.objects[id]);
        if (!ids.length) {
          gesture.current = null;
          return;
        }
        st.beginGesture();
        gesture.current = { kind: "move", sx: g.sx, sy: g.sy, ids };
        setInteracting(true);
        return onPointerMove(e);
      }
      case "move": {
        const dx = (sx - g.sx) / c.zoom;
        const dy = (sy - g.sy) / c.zoom;
        st.gesture((base) => {
          const patches: Record<string, Partial<BoardObject>> = {};
          for (const id of g.ids) {
            const o = base.objects[id];
            if (o) patches[id] = { x: o.x + dx, y: o.y + dy };
          }
          return updateObjects(base, patches);
        });
        return;
      }
      case "resize": {
        const dx = (sx - g.sx) / c.zoom;
        const dy = (sy - g.sy) / c.zoom;
        const next = resizeRect(g.box, g.handle, dx, dy, g.keepAspect || e.shiftKey);
        st.gesture((base) => {
          const patches: Record<string, Partial<BoardObject>> = {};
          for (const id of g.ids) {
            const o = base.objects[id];
            if (o) patches[id] = mapRect(rectOf(o), g.box, next);
          }
          return updateObjects(base, patches);
        });
        return;
      }
      case "marquee": {
        const cur = screenToWorld(c, sx, sy);
        const r = rectFromPoints(g.start, cur);
        const hits = Object.values(st.doc.objects)
          .filter((o) => intersects(rectOf(o), r))
          .map((o) => o.id);
        st.select(expandToGroups(st.doc, [...new Set([...g.base, ...hits])]));
        const a = { x: g.start.x * c.zoom + c.x, y: g.start.y * c.zoom + c.y };
        setMarquee(rectFromPoints(a, { x: sx, y: sy }));
        return;
      }
    }
  };

  const endGesture = (e: PointerEvent<HTMLDivElement>) => {
    const g = gesture.current;
    gesture.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    if (!g) return;
    const st = useBoard.getState();
    if (g.kind === "pending" && !g.shift && g.wasSelected && st.selection.length > g.unit.length) {
      // Clic (sin arrastrar) en un elemento de una selección múltiple: queda solo él.
      st.select(g.unit);
    }
    if (g.kind === "move" || g.kind === "resize") st.endGesture();
    if (g.kind === "marquee") setMarquee(null);
    if (g.kind === "pan") setPanMode(space.current ? "ready" : "none");
    setInteracting(false);
  };

  const onDoubleClick = (e: MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    if (target.closest("[data-ui]") || space.current) return;
    const st = useBoard.getState();
    const objEl = target.closest<HTMLElement>("[data-object-id]");
    if (objEl) {
      const id = objEl.dataset.objectId!;
      const o = st.doc.objects[id];
      if (!o || st.editingId === id) return;
      if (o.groupId && st.isolatedGroup !== o.groupId) {
        // Doble clic dentro de un grupo: editar ese elemento por separado.
        st.setIsolated(o.groupId);
        st.select([id]);
        return;
      }
      if (o.kind === "note") {
        st.select([id]);
        st.setEditing(id);
      }
      return;
    }
    const { sx, sy } = local(e.clientX, e.clientY);
    addNote(screenToWorld(camera.current, sx, sy));
  };

  const onContextMenu = (e: MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    const target = e.target as HTMLElement;
    if (target.closest("[data-ui]")) return;
    const st = useBoard.getState();
    const objEl = target.closest<HTMLElement>("[data-object-id]");
    if (objEl) {
      const id = objEl.dataset.objectId!;
      if (!st.selection.includes(id)) st.select(expandToGroups(st.doc, [id], st.isolatedGroup));
      setMenu({ kind: "object", x: e.clientX, y: e.clientY });
    } else {
      const { sx, sy } = local(e.clientX, e.clientY);
      setMenu({ kind: "canvas", x: e.clientX, y: e.clientY, world: screenToWorld(camera.current, sx, sy) });
    }
  };

  // Rueda: desplazamiento; Ctrl + rueda (o pellizco del touchpad): zoom al cursor.
  useEffect(() => {
    const vp = viewportRef.current;
    if (!vp) return;
    const onWheel = (e: WheelEvent) => {
      if ((e.target as HTMLElement).closest("textarea, [data-ui]")) return;
      e.preventDefault();
      const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? vp.clientHeight : 1;
      const dx = e.deltaX * unit;
      const dy = e.deltaY * unit;
      const c = camera.current;
      if (e.ctrlKey || e.metaKey) {
        const { sx, sy } = local(e.clientX, e.clientY);
        const factor = Math.exp(-Math.max(-120, Math.min(120, dy)) * 0.0022);
        camera.set(zoomAt(c, c.zoom * factor, sx, sy));
      } else if (e.shiftKey && !dx) {
        camera.set({ ...c, x: c.x - dy });
      } else {
        camera.set({ ...c, x: c.x - dx, y: c.y - dy });
      }
    };
    vp.addEventListener("wheel", onWheel, { passive: false });
    return () => vp.removeEventListener("wheel", onWheel);
  }, [viewportRef, local]);

  // Espacio mantenido = modo desplazamiento.
  const setSpace = useCallback((down: boolean) => {
    space.current = down;
    if (gesture.current?.kind !== "pan") setPanMode(down ? "ready" : "none");
  }, []);

  // Soltar capturas (desde el panel o el cajón) o archivos de imagen.
  const accepts = (e: DragEvent) => e.dataTransfer.types.includes(CAPTURE_DRAG_TYPE) || e.dataTransfer.types.includes("Files");
  const onDragOver = (e: DragEvent<HTMLDivElement>) => {
    if (!accepts(e)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setDropActive(true);
  };
  const onDragLeave = (e: DragEvent<HTMLDivElement>) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node)) setDropActive(false);
  };
  const onDrop = async (e: DragEvent<HTMLDivElement>) => {
    setDropActive(false);
    if (!accepts(e)) return;
    e.preventDefault();
    const { sx, sy } = local(e.clientX, e.clientY);
    const at = screenToWorld(camera.current, sx, sy);
    const id = e.dataTransfer.getData(CAPTURE_DRAG_TYPE);
    if (id) {
      const [capture] = await api.getCaptures([id]);
      if (capture) addCapture(capture, at);
      return;
    }
    if (e.dataTransfer.files.length) await importFiles([...e.dataTransfer.files], at);
  };

  return {
    handlers: {
      onPointerDown,
      onPointerMove,
      onPointerUp: endGesture,
      onPointerCancel: endGesture,
      onDoubleClick,
      onContextMenu,
      onDragOver,
      onDragLeave,
      onDrop,
    },
    marquee,
    interacting,
    panMode,
    setSpace,
    menu,
    closeMenu: () => setMenu(null),
    dropActive,
  };
}
