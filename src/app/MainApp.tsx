import { useCallback, useEffect } from "react";

import { Canvas } from "@/components/canvas/Canvas";
import { CaptureList } from "@/components/clipboard/CaptureList";
import { Flash } from "@/components/common/Flash";
import { SettingsSheet } from "@/components/settings/SettingsSheet";
import { placeIncoming } from "@/features/canvas/actions";
import { useBoard } from "@/features/canvas/boardStore";
import { flushSave } from "@/features/canvas/persistence";
import { on } from "@/services/events";
import { useBoards } from "@/stores/boardsStore";
import { flash } from "@/stores/flashStore";
import { useUi } from "@/stores/uiStore";

import { TitleBar } from "./TitleBar";

export function MainApp() {
  const settingsOpen = useUi((s) => s.settingsOpen);
  const drawerOpen = useUi((s) => s.drawerOpen);
  const activeId = useBoards((s) => s.activeId);

  // Abrir el pizarrón activo (y recordar el último usado).
  useEffect(() => {
    if (activeId && useBoard.getState().boardId !== activeId) void useBoard.getState().load(activeId);
  }, [activeId]);

  const openBoard = useCallback(async (id: string) => {
    await flushSave();
    try {
      await useBoards.getState().setActive(id);
    } catch {
      flash("No se pudo abrir el pizarrón", "error");
    }
  }, []);

  useEffect(() => {
    const subs = [
      on("board:objects-inserted", async (e) => {
        if (e.boardId !== useBoard.getState().boardId) return;
        await useBoard.getState().ensureCaptures(e.objects.map((o) => o.captureId).filter((id): id is string => !!id));
        placeIncoming(e.objects);
      }),
      on("captures:changed", (e) => {
        if (e.removed.length) useBoard.getState().dropCaptures(e.removed);
        if (e.updated) useBoard.getState().setCapture(e.updated);
      }),
      on("main:navigate", (e) => useUi.getState().setSettingsOpen(e.view === "settings")),
      on("app:flush", () => void flushSave()),
    ];
    return () => subs.forEach((p) => void p.then((f) => f()));
  }, []);

  return (
    <div className="app">
      <TitleBar onOpenBoard={openBoard} />
      <div className="app-body">
        <Canvas />
        {drawerOpen && (
          <aside className="drawer" aria-label="Historial">
            <div className="drawer-title">Historial</div>
            <CaptureList keyboard={false} emptyHint="Las capturas aparecerán acá. Arrastralas al pizarrón para organizarlas." />
          </aside>
        )}
      </div>
      {settingsOpen && <SettingsSheet />}
      <Flash />
    </div>
  );
}
