// Estado del pizarrón abierto: documento, selección, edición y deshacer.

import { create } from "zustand";

import { api, errorMessage } from "@/services/api";
import type { BoardObject, Capture } from "@/types/models";

import { camera } from "./camera";
import { emptyDoc, expandToGroups, fromBoardDocument, normalizeGroups, type Doc } from "./document";
import { emptyUndo, record, redo as redoStep, undo as undoStep, type UndoState } from "./history";

export interface BoardState {
  boardId: string | null;
  boardName: string;
  status: "idle" | "loading" | "ready" | "error";
  error: string | null;

  doc: Doc;
  /** Último estado confirmado en la base de datos. */
  saved: Doc;
  history: UndoState<Doc>;
  /** Base de un gesto en curso (arrastre/redimensión): un solo paso de deshacer. */
  gestureBase: Doc | null;

  captures: Record<string, Capture>;
  selection: string[];
  /** Grupo "abierto" para editar sus miembros por separado. */
  isolatedGroup: string | null;
  editingId: string | null;

  load: (boardId: string) => Promise<void>;
  commit: (fn: (d: Doc) => Doc) => void;
  beginGesture: () => void;
  gesture: (fn: (base: Doc) => Doc) => void;
  endGesture: () => void;
  undo: () => void;
  redo: () => void;
  select: (ids: string[]) => void;
  setIsolated: (groupId: string | null) => void;
  setEditing: (id: string | null) => void;
  markSaved: (doc: Doc, skipped: string[]) => void;
  ensureCaptures: (ids: string[]) => Promise<void>;
  /** Integra objetos ya guardados por Rust (agregados desde el panel). */
  mergeSaved: (objects: BoardObject[]) => void;
  /** Quita objetos cuyas capturas se borraron permanentemente. */
  dropCaptures: (captureIds: string[]) => void;
  setCapture: (c: Capture) => void;
}

function pruneSelection(doc: Doc, ids: string[]): string[] {
  return ids.filter((id) => doc.objects[id]);
}

export const useBoard = create<BoardState>((set, get) => ({
  boardId: null,
  boardName: "",
  status: "idle",
  error: null,
  doc: emptyDoc(),
  saved: emptyDoc(),
  history: emptyUndo(),
  gestureBase: null,
  captures: {},
  selection: [],
  isolatedGroup: null,
  editingId: null,

  load: async (boardId) => {
    set({ status: "loading", boardId, error: null, selection: [], editingId: null, isolatedGroup: null });
    try {
      const d = await api.loadBoard(boardId);
      if (get().boardId !== boardId) return;
      const doc = fromBoardDocument(d);
      const ids = [...new Set(d.objects.map((o) => o.captureId).filter((id): id is string => !!id))];
      const caps = ids.length ? await api.getCaptures(ids) : [];
      if (get().boardId !== boardId) return;
      const captures: Record<string, Capture> = {};
      for (const c of caps) captures[c.id] = c;
      const isDefault = d.board.cameraX === 0 && d.board.cameraY === 0 && d.board.zoom === 1;
      const { width, height } = camera.viewport;
      // Sin cámara guardada: origen del mundo en el centro de la vista.
      camera.set(
        isDefault && width > 1
          ? { x: width / 2, y: height / 2, zoom: 1 }
          : { x: d.board.cameraX, y: d.board.cameraY, zoom: d.board.zoom || 1 },
      );
      set({
        boardName: d.board.name,
        status: "ready",
        doc,
        saved: doc,
        history: emptyUndo(),
        gestureBase: null,
        captures,
      });
    } catch (e) {
      if (get().boardId === boardId) set({ status: "error", error: errorMessage(e) });
    }
  },

  commit: (fn) => {
    const { doc, history, gestureBase } = get();
    if (gestureBase) return; // nunca mezclar con un gesto en curso
    const next = fn(doc);
    if (next === doc) return;
    set({ doc: next, history: record(history, doc), selection: pruneSelection(next, get().selection) });
  },

  beginGesture: () => {
    if (!get().gestureBase) set({ gestureBase: get().doc });
  },
  gesture: (fn) => {
    const base = get().gestureBase;
    if (base) set({ doc: fn(base) });
  },
  endGesture: () => {
    const { gestureBase, doc, history } = get();
    if (!gestureBase) return;
    set({ gestureBase: null, history: doc === gestureBase ? history : record(history, gestureBase) });
  },

  undo: () => {
    const { history, doc, gestureBase } = get();
    if (gestureBase) return;
    const r = undoStep(history, doc);
    if (r) set({ doc: r.state, history: r.history, selection: pruneSelection(r.state, get().selection), editingId: null });
  },
  redo: () => {
    const { history, doc, gestureBase } = get();
    if (gestureBase) return;
    const r = redoStep(history, doc);
    if (r) set({ doc: r.state, history: r.history, selection: pruneSelection(r.state, get().selection), editingId: null });
  },

  select: (ids) => set({ selection: ids }),
  setIsolated: (groupId) => set({ isolatedGroup: groupId }),
  setEditing: (id) => set({ editingId: id }),

  markSaved: (snapshot, skipped) => {
    if (skipped.length === 0) {
      set({ saved: snapshot });
      return;
    }
    // Objetos que la base rechazó (su captura ya no existe): fuera del documento.
    const drop = (d: Doc): Doc => {
      const objects = { ...d.objects };
      for (const id of skipped) delete objects[id];
      return normalizeGroups({ ...d, objects });
    };
    set((s) => ({ saved: drop(snapshot), doc: drop(s.doc), history: emptyUndo() }));
  },

  ensureCaptures: async (ids) => {
    const missing = ids.filter((id) => !get().captures[id]);
    if (!missing.length) return;
    const caps = await api.getCaptures(missing);
    set((s) => {
      const captures = { ...s.captures };
      for (const c of caps) captures[c.id] = c;
      return { captures };
    });
  },

  mergeSaved: (objects) => {
    set((s) => {
      const add = (d: Doc): Doc => {
        const objs = { ...d.objects };
        for (const o of objects) objs[o.id] = o;
        return { ...d, objects: objs };
      };
      return { doc: add(s.doc), saved: add(s.saved), history: record(s.history, s.doc) };
    });
  },

  dropCaptures: (captureIds) => {
    const gone = new Set(captureIds);
    const { doc } = get();
    if (!Object.values(doc.objects).some((o) => o.captureId && gone.has(o.captureId))) return;
    const strip = (d: Doc): Doc => {
      const objects = { ...d.objects };
      for (const o of Object.values(objects)) if (o.captureId && gone.has(o.captureId)) delete objects[o.id];
      return normalizeGroups({ ...d, objects });
    };
    // Rust ya borró estas filas; el historial de deshacer las traería de vuelta.
    set((s) => ({ doc: strip(s.doc), saved: strip(s.saved), history: emptyUndo(), selection: pruneSelection(strip(s.doc), s.selection) }));
  },

  setCapture: (c) => {
    if (get().captures[c.id]) set((s) => ({ captures: { ...s.captures, [c.id]: c } }));
  },
}));

/** Selección expandida a grupos según el modo de aislamiento actual. */
export function selectExpanded(ids: string[]) {
  const s = useBoard.getState();
  s.select(expandToGroups(s.doc, ids, s.isolatedGroup));
}
