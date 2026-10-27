// Guardado automático: cada cambio del documento programa un guardado
// incremental (diff contra lo último confirmado). Un solo guardado en vuelo;
// si falla se reintenta. La cámara se guarda aparte, con menos frecuencia.

import { api, errorMessage } from "@/services/api";
import { flash } from "@/stores/flashStore";
import { debounce } from "@/utils/timing";

import { useBoard } from "./boardStore";
import { camera } from "./camera";
import { diffDocs, isEmptyChange } from "./document";

const SAVE_DELAY = 350;
const CAMERA_DELAY = 700;
const RETRY_DELAY = 2500;

let inflight: Promise<void> | null = null;
let again = false;
let failures = 0;

async function saveNow(): Promise<void> {
  if (inflight) {
    again = true;
    return inflight;
  }
  const { boardId, doc, saved, status, gestureBase } = useBoard.getState();
  if (!boardId || status !== "ready" || gestureBase) return;
  const changes = diffDocs(saved, doc, boardId);
  if (isEmptyChange(changes)) return;

  inflight = (async () => {
    try {
      const report = await api.saveBoardChanges(changes);
      if (useBoard.getState().boardId === boardId) useBoard.getState().markSaved(doc, report.skippedObjects);
      failures = 0;
    } catch (e) {
      failures++;
      if (failures === 1 || failures % 5 === 0) flash(`No se pudieron guardar los cambios: ${errorMessage(e)}`, "error");
      setTimeout(() => scheduleSave(), RETRY_DELAY);
    }
  })();
  await inflight;
  inflight = null;
  if (again) {
    again = false;
    await saveNow();
  }
}

const scheduleSave = debounce(() => void saveNow(), SAVE_DELAY);

const saveCamera = debounce((boardId: string) => {
  const c = camera.current;
  api.saveCamera(boardId, c.x, c.y, c.zoom).catch(() => {});
}, CAMERA_DELAY);

/** Guarda todo lo pendiente ya (al cambiar de pizarrón, ocultar o salir). */
export async function flushSave(): Promise<void> {
  scheduleSave.flush();
  saveCamera.flush();
  await saveNow();
  if (inflight) await inflight;
}

export function initPersistence() {
  useBoard.subscribe((s, prev) => {
    if (s.status !== "ready") return;
    const docChanged = s.doc !== prev.doc && s.boardId === prev.boardId;
    const gestureEnded = prev.gestureBase && !s.gestureBase;
    if ((docChanged && !s.gestureBase) || gestureEnded) scheduleSave();
  });
  camera.subscribe(() => {
    const { boardId, status } = useBoard.getState();
    if (boardId && status === "ready") saveCamera(boardId);
  });
  window.addEventListener("beforeunload", () => void flushSave());
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") void flushSave();
  });
}
