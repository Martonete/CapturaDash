// Acciones de alto nivel del pizarrón, compartidas por atajos, menús y barras.

import { api, errorMessage } from "@/services/api";
import { flash } from "@/stores/flashStore";
import type { BoardObject, Capture, NoteColor } from "@/types/models";
import { newId } from "@/utils/ids";

import { useBoard } from "./boardStore";
import { camera } from "./camera";
import {
  addObjects,
  expandToGroups,
  groupObjects,
  maxZ,
  rectOf,
  removeObjects,
  reorder,
  setNoteColor,
  ungroupObjects,
} from "./document";
import { boundsOf, fitCamera, screenToWorld, zoomAt, type Point } from "./geometry";

/** Lado máximo inicial de una imagen nueva en el pizarrón. */
const MAX_INITIAL_SIDE = 900;
export const NOTE_SIZE = { width: 220, height: 168 };

/** Última posición del puntero en coordenadas del mundo (para pegar). */
let lastPointer: Point | null = null;
export const setLastPointer = (p: Point | null) => (lastPointer = p);

export function viewportCenter(): Point {
  const { width, height } = camera.viewport;
  return screenToWorld(camera.current, width / 2, height / 2);
}

function baseObject(kind: BoardObject["kind"]): Omit<BoardObject, "x" | "y" | "width" | "height"> {
  const s = useBoard.getState();
  const now = Date.now();
  return {
    id: newId(),
    boardId: s.boardId!,
    kind,
    captureId: null,
    z: maxZ(s.doc) + 1,
    groupId: null,
    props: {},
    createdAt: now,
    updatedAt: now,
  };
}

/** Tamaño "1:1" en pantalla: los píxeles de la captura divididos por la escala. */
export function initialImageSize(c: Capture): { width: number; height: number } {
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  let w = c.width / dpr;
  let h = c.height / dpr;
  const shrink = Math.min(1, MAX_INITIAL_SIDE / Math.max(w, h));
  w *= shrink;
  h *= shrink;
  return { width: w, height: h };
}

export function addCapture(c: Capture, at?: Point) {
  const s = useBoard.getState();
  if (!s.boardId || s.status !== "ready") return;
  useBoard.setState((st) => ({ captures: { ...st.captures, [c.id]: c } }));
  const p = at ?? viewportCenter();
  const { width, height } = initialImageSize(c);
  const obj: BoardObject = { ...baseObject("image"), captureId: c.id, x: p.x - width / 2, y: p.y - height / 2, width, height };
  s.commit((d) => addObjects(d, [obj]));
  useBoard.getState().select([obj.id]);
}

export function addNote(at?: Point, text = "") {
  const s = useBoard.getState();
  if (!s.boardId || s.status !== "ready") return;
  const p = at ?? viewportCenter();
  const obj: BoardObject = {
    ...baseObject("note"),
    x: p.x - NOTE_SIZE.width / 2,
    y: p.y - NOTE_SIZE.height / 2,
    ...NOTE_SIZE,
    props: { text, color: "sand" },
  };
  s.commit((d) => addObjects(d, [obj]));
  useBoard.setState({ selection: [obj.id], editingId: text ? null : obj.id });
}

export function deleteSelection() {
  const { selection, commit } = useBoard.getState();
  if (!selection.length) return;
  commit((d) => removeObjects(d, selection));
  useBoard.setState({ selection: [], isolatedGroup: null, editingId: null });
}

export function groupSelection() {
  const { selection, boardId, commit } = useBoard.getState();
  if (selection.length < 2 || !boardId) return;
  commit((d) => groupObjects(d, selection, { id: newId(), boardId, createdAt: Date.now() }));
  useBoard.setState({ isolatedGroup: null });
}

export function ungroupSelection() {
  const { selection, commit } = useBoard.getState();
  if (!selection.length) return;
  commit((d) => ungroupObjects(d, selection));
  useBoard.setState({ isolatedGroup: null });
}

export function selectionIsGroup(): boolean {
  const { selection, doc } = useBoard.getState();
  return selection.some((id) => doc.objects[id]?.groupId);
}

export function reorderSelection(where: "front" | "back") {
  const { selection, commit } = useBoard.getState();
  if (selection.length) commit((d) => reorder(d, selection, where));
}

export function colorSelection(color: NoteColor) {
  const { selection, commit } = useBoard.getState();
  commit((d) => setNoteColor(d, selection, color));
}

export function selectAll() {
  const { doc } = useBoard.getState();
  useBoard.setState({ selection: Object.keys(doc.objects), isolatedGroup: null });
}

/** Ctrl+C: imagen → portapapeles (sin duplicar en el historial); nota → texto. */
export async function copySelection() {
  const { selection, doc } = useBoard.getState();
  const objs = selection.map((id) => doc.objects[id]).filter(Boolean);
  const image = objs.find((o) => o.kind === "image" && o.captureId);
  try {
    if (image) {
      await api.copyCapture(image.captureId!);
      flash(objs.length > 1 ? "Imagen principal copiada" : "Imagen copiada");
    } else {
      const text = objs.map((o) => o.props.text ?? "").filter(Boolean).join("\n\n");
      if (!text) return;
      await api.copyText(text);
      flash("Texto copiado");
    }
  } catch (e) {
    flash(errorMessage(e), "error");
  }
}

/** Ctrl+V: imagen del portapapeles → objeto; texto → nota. */
export async function pasteFromClipboard() {
  try {
    const r = await api.pasteClipboard();
    const at = lastPointer ?? undefined;
    if (r.kind === "Image") addCapture(r.capture, at);
    else if (r.kind === "Text") addNote(at, r.text.slice(0, 5000));
    else flash("El portapapeles no tiene una imagen ni texto");
  } catch (e) {
    flash(errorMessage(e), "error");
  }
}

/** Archivos de imagen soltados desde el Explorador. */
export async function importFiles(files: File[], at: Point) {
  let offset = 0;
  for (const f of files) {
    if (!/^image\/(png|jpe?g)$/.test(f.type)) continue;
    try {
      const capture = await api.importImageBytes(new Uint8Array(await f.arrayBuffer()));
      addCapture(capture, { x: at.x + offset, y: at.y + offset });
      offset += 32;
    } catch (e) {
      flash(`${f.name}: ${errorMessage(e)}`, "error");
    }
  }
}

export function fitAll(animated = true) {
  const { doc, selection } = useBoard.getState();
  const ids = selection.length ? selection : Object.keys(doc.objects);
  const b = boundsOf(ids.map((id) => doc.objects[id]).filter(Boolean).map(rectOf));
  const { width, height } = camera.viewport;
  const target = b ? fitCamera(b, width, height, 72, 1) : { x: width / 2, y: height / 2, zoom: 1 };
  if (animated) camera.animateTo(target);
  else camera.set(target);
}

export function zoomBy(factor: number) {
  const c = camera.current;
  const { width, height } = camera.viewport;
  camera.animateTo(zoomAt(c, c.zoom * factor, width / 2, height / 2), 160);
}

export function resetZoom() {
  const c = camera.current;
  const { width, height } = camera.viewport;
  camera.animateTo(zoomAt(c, 1, width / 2, height / 2), 160);
}

/** Objetos que entraron desde el panel: los trae a la vista y los selecciona. */
export function placeIncoming(objects: BoardObject[]) {
  const s = useBoard.getState();
  s.mergeSaved(objects);
  const center = viewportCenter();
  const patches: Record<string, Partial<BoardObject>> = {};
  objects.forEach((o, i) => {
    patches[o.id] = { x: center.x - o.width / 2 + i * 24, y: center.y - o.height / 2 + i * 24 };
  });
  // La reubicación no suma un paso de deshacer: es parte de la misma inserción.
  useBoard.setState((st) => {
    const objs = { ...st.doc.objects };
    for (const [id, p] of Object.entries(patches)) if (objs[id]) objs[id] = { ...objs[id], ...p, updatedAt: Date.now() };
    return { doc: { ...st.doc, objects: objs }, selection: expandToGroups(st.doc, objects.map((o) => o.id)) };
  });
}
