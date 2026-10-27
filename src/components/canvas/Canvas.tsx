import {
  ArrowDownToLine,
  ArrowUpToLine,
  ClipboardPaste,
  Copy,
  Group,
  ImagePlus,
  Maximize,
  StickyNote,
  Trash2,
  Ungroup,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import {
  addNote,
  copySelection,
  deleteSelection,
  fitAll,
  groupSelection,
  pasteFromClipboard,
  reorderSelection,
  ungroupSelection,
} from "@/features/canvas/actions";
import { useBoard } from "@/features/canvas/boardStore";
import { camera, useCameraSnapshot } from "@/features/canvas/camera";
import { rectOf, sortedObjects } from "@/features/canvas/document";
import { intersects, visibleWorld, type Camera } from "@/features/canvas/geometry";
import { useCanvasInput } from "@/features/canvas/useCanvasInput";
import { useCanvasKeyboard } from "@/features/canvas/useCanvasKeyboard";
import { useSettings } from "@/stores/settingsStore";
import { useUi } from "@/stores/uiStore";

import { Menu, type MenuItem } from "../common/Menu";
import { CanvasHud } from "./CanvasHud";
import { ContextToolbar } from "./ContextToolbar";
import { ObjectView } from "./ObjectView";
import { SelectionLayer } from "./SelectionLayer";

const GRID = 24;

/** Pizarrón infinito: DOM con una única transformación 2D para todo el mundo. */
export function Canvas() {
  const viewportRef = useRef<HTMLDivElement>(null);
  const worldRef = useRef<HTMLDivElement>(null);
  const status = useBoard((s) => s.status);
  const error = useBoard((s) => s.error);
  const doc = useBoard((s) => s.doc);
  const captures = useBoard((s) => s.captures);
  const selection = useBoard((s) => s.selection);
  const editingId = useBoard((s) => s.editingId);
  const grid = useSettings((s) => s.settings?.grid ?? "dots");
  const snap = useCameraSnapshot(100);
  const [size, setSize] = useState(camera.viewport);
  const input = useCanvasInput(viewportRef);
  useCanvasKeyboard(input.setSpace);

  // Cámara → DOM, sin pasar por React. Mientras se mueve, `will-change`
  // mantiene la capa en GPU; al detenerse se quita para re-rasterizar nítido.
  useEffect(() => {
    const vp = viewportRef.current!;
    const world = worldRef.current!;
    let idle: ReturnType<typeof setTimeout> | undefined;
    let frame = 0;
    let pending: Camera | null = null;
    const apply = () => {
      frame = 0;
      const c = pending!;
      world.style.transform = `translate3d(${c.x}px, ${c.y}px, 0) scale(${c.zoom})`;
      world.style.setProperty("--zoom", String(c.zoom));
      let step = GRID * c.zoom;
      while (step < 14) step *= 2;
      while (step > 48) step /= 2;
      vp.style.backgroundSize = `${step}px ${step}px`;
      vp.style.backgroundPosition = `${c.x}px ${c.y}px`;
      world.classList.add("is-moving");
      clearTimeout(idle);
      idle = setTimeout(() => world.classList.remove("is-moving"), 160);
    };
    const onCamera = (c: Camera) => {
      pending = c;
      if (!frame) frame = requestAnimationFrame(apply);
    };
    onCamera(camera.current);
    const unsub = camera.subscribe(onCamera);

    const ro = new ResizeObserver(([entry]) => {
      const first = camera.viewport.width <= 1;
      const next = { width: entry.contentRect.width, height: entry.contentRect.height };
      camera.viewport = next;
      setSize(next);
      // Pizarrón sin cámara guardada: el origen del mundo va al centro de la vista.
      const c = camera.current;
      if (first && c.x === 0 && c.y === 0 && c.zoom === 1) camera.set({ x: next.width / 2, y: next.height / 2, zoom: 1 });
    });
    ro.observe(vp);
    return () => {
      unsub();
      ro.disconnect();
      cancelAnimationFrame(frame);
      clearTimeout(idle);
    };
  }, []);

  const ordered = useMemo(() => sortedObjects(doc), [doc]);
  const selected = useMemo(() => new Set(selection), [selection]);

  // Solo se renderiza lo que está cerca de la vista (más lo seleccionado).
  const view = visibleWorld(snap, size.width, size.height, 0.5);
  const dpr = window.devicePixelRatio || 1;
  const hiRes = useRef(new Set<string>());
  const visible = ordered.filter((o) => selected.has(o.id) || o.id === editingId || intersects(rectOf(o), view));

  const wantsOriginal = (id: string, displayPx: number, thumbW: number, fullW: number) => {
    if (fullW <= thumbW) return false;
    // Histéresis para no alternar entre miniatura y original en el umbral.
    const on = hiRes.current.has(id) ? displayPx > thumbW * 0.85 : displayPx > thumbW * 1.15;
    if (on) hiRes.current.add(id);
    else hiRes.current.delete(id);
    return on;
  };

  const menuItems = (): MenuItem[] => {
    const m = input.menu!;
    if (m.kind === "canvas") {
      return [
        { id: "paste", label: "Pegar", icon: <ClipboardPaste size={14} />, shortcut: "Ctrl+V", onSelect: () => void pasteFromClipboard() },
        { id: "note", label: "Nueva nota", icon: <StickyNote size={14} />, onSelect: () => addNote(m.world) },
        { id: "history", label: "Mostrar historial", icon: <ImagePlus size={14} />, onSelect: () => !useUi.getState().drawerOpen && useUi.getState().toggleDrawer() },
        { id: "fit", label: "Encuadrar todo", icon: <Maximize size={14} />, shortcut: "F", separator: true, onSelect: () => fitAll() },
      ];
    }
    const objs = selection.map((id) => doc.objects[id]).filter(Boolean);
    const grouped = objs.some((o) => o.groupId);
    return [
      { id: "copy", label: "Copiar", icon: <Copy size={14} />, shortcut: "Ctrl+C", onSelect: () => void copySelection() },
      { id: "group", label: "Agrupar", icon: <Group size={14} />, shortcut: "Ctrl+G", disabled: objs.length < 2, onSelect: groupSelection },
      { id: "ungroup", label: "Desagrupar", icon: <Ungroup size={14} />, shortcut: "Ctrl+Shift+G", disabled: !grouped, onSelect: ungroupSelection },
      { id: "front", label: "Traer al frente", icon: <ArrowUpToLine size={14} />, separator: true, onSelect: () => reorderSelection("front") },
      { id: "back", label: "Enviar al fondo", icon: <ArrowDownToLine size={14} />, onSelect: () => reorderSelection("back") },
      { id: "delete", label: "Quitar del pizarrón", icon: <Trash2 size={14} />, shortcut: "Supr", danger: true, separator: true, onSelect: deleteSelection },
    ];
  };

  const empty = status === "ready" && ordered.length === 0;

  return (
    <div
      ref={viewportRef}
      className={`viewport grid-${grid} pan-${input.panMode} ${input.dropActive ? "is-drop" : ""}`}
      {...input.handlers}
    >
      <div ref={worldRef} className="world">
        {visible.map((o) => {
          const c = o.captureId ? captures[o.captureId] : undefined;
          return (
            <ObjectView
              key={o.id}
              obj={o}
              capture={c}
              selected={selected.has(o.id)}
              editing={o.id === editingId}
              hiRes={!!c && wantsOriginal(o.id, o.width * snap.zoom * dpr, c.thumbWidth, c.width)}
            />
          );
        })}
        <SelectionLayer hideHandles={input.interacting} />
        {!input.interacting && <ContextToolbar />}
      </div>

      {input.marquee && (
        <div
          className="marquee"
          style={{ left: input.marquee.x, top: input.marquee.y, width: input.marquee.width, height: input.marquee.height }}
        />
      )}

      {empty && (
        <div className="canvas-empty">
          <p className="canvas-empty-title">Pizarrón vacío</p>
          <p>
            Arrastrá capturas desde el historial, pegá con <kbd>Ctrl</kbd> <kbd>V</kbd> o hacé doble clic para crear una nota.
          </p>
        </div>
      )}
      {status === "loading" && <div className="canvas-status">Cargando…</div>}
      {status === "error" && <div className="canvas-status is-error">{error}</div>}

      <CanvasHud />
      {input.menu && <Menu x={input.menu.x} y={input.menu.y} items={menuItems()} onClose={input.closeMenu} />}
    </div>
  );
}
