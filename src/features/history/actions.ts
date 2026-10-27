// Acciones sobre capturas del historial (panel y cajón del pizarrón).

import { api, errorMessage } from "@/services/api";
import { flash } from "@/stores/flashStore";
import type { Capture } from "@/types/models";

export async function copyCapture(c: Capture): Promise<boolean> {
  try {
    await api.copyCapture(c.id);
    return true;
  } catch (e) {
    flash(errorMessage(e), "error");
    return false;
  }
}

export async function addToBoard(c: Capture, boardId?: string) {
  try {
    const r = await api.addCaptureToBoard(c.id, boardId);
    flash(`Agregada a “${r.boardName}”`);
  } catch (e) {
    flash(errorMessage(e), "error");
  }
}

export async function toggleFavorite(c: Capture) {
  try {
    await api.setFavorite(c.id, !c.favorite);
  } catch (e) {
    flash(errorMessage(e), "error");
  }
}

/** Borrado permanente (archivo incluido). `force` también la quita de los pizarrones. */
export async function deleteCapture(c: Capture, force: boolean) {
  try {
    await api.deleteCapture(c.id, force);
  } catch (e) {
    flash(errorMessage(e), "error");
  }
}
