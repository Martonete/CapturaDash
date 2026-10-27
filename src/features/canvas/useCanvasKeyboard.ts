// Atajos del pizarrón. Son contextuales: no actúan mientras se escribe en una
// nota, en un campo de texto o con un menú/hoja abiertos.

import { useEffect } from "react";

import { useUi } from "@/stores/uiStore";
import { isEditableTarget } from "@/utils/timing";

import {
  copySelection,
  deleteSelection,
  fitAll,
  groupSelection,
  pasteFromClipboard,
  resetZoom,
  selectAll,
  ungroupSelection,
  zoomBy,
} from "./actions";
import { useBoard } from "./boardStore";

export function useCanvasKeyboard(setSpace: (down: boolean) => void) {
  useEffect(() => {
    const blocked = (e: KeyboardEvent) =>
      isEditableTarget(e.target) || useUi.getState().settingsOpen || !!document.querySelector(".menu, .popover");

    const onDown = (e: KeyboardEvent) => {
      if (blocked(e)) return;
      const ctrl = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      const st = useBoard.getState();

      if (e.code === "Space" && !ctrl) {
        e.preventDefault();
        if (!e.repeat) setSpace(true);
        return;
      }
      if (ctrl && key === "z" && !e.shiftKey) return void (e.preventDefault(), st.undo());
      if (ctrl && ((key === "z" && e.shiftKey) || key === "y")) return void (e.preventDefault(), st.redo());
      if (ctrl && key === "c") return void (e.preventDefault(), copySelection());
      if (ctrl && key === "v") return void (e.preventDefault(), pasteFromClipboard());
      if (ctrl && key === "g" && e.shiftKey) return void (e.preventDefault(), ungroupSelection());
      if (ctrl && key === "g") return void (e.preventDefault(), groupSelection());
      if (ctrl && key === "a") return void (e.preventDefault(), selectAll());
      if (ctrl && key === "0") return void (e.preventDefault(), resetZoom());
      if (ctrl && (key === "=" || key === "+")) return void (e.preventDefault(), zoomBy(1.25));
      if (ctrl && key === "-") return void (e.preventDefault(), zoomBy(0.8));
      if (ctrl) return;

      if (key === "delete" || key === "backspace") return void (e.preventDefault(), deleteSelection());
      if (key === "f") return void (e.preventDefault(), fitAll());
      if (key === "escape") {
        if (st.isolatedGroup) st.setIsolated(null);
        st.select([]);
        return;
      }
      if (key === "enter" && st.selection.length === 1) {
        const o = st.doc.objects[st.selection[0]];
        if (o?.kind === "note") {
          e.preventDefault();
          st.setEditing(o.id);
        }
      }
    };
    const onUp = (e: KeyboardEvent) => {
      if (e.code === "Space") setSpace(false);
    };
    const onBlur = () => setSpace(false);
    window.addEventListener("keydown", onDown);
    window.addEventListener("keyup", onUp);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onDown);
      window.removeEventListener("keyup", onUp);
      window.removeEventListener("blur", onBlur);
    };
  }, [setSpace]);
}
