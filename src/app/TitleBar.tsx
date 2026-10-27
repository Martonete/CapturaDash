import { getCurrentWindow } from "@tauri-apps/api/window";
import { History, Minus, Settings as SettingsIcon, Square, X } from "lucide-react";

import { BoardSwitcher } from "@/components/boards/BoardSwitcher";
import { IconButton } from "@/components/common/IconButton";
import { useUi } from "@/stores/uiStore";

/** Barra de título propia: arrastrable, compacta y sin menús permanentes. */
export function TitleBar({ onOpenBoard }: { onOpenBoard: (id: string) => void }) {
  const drawerOpen = useUi((s) => s.drawerOpen);
  const toggleDrawer = useUi((s) => s.toggleDrawer);
  const openSettings = () => useUi.getState().setSettingsOpen(true);
  const win = getCurrentWindow();

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar-left" data-tauri-drag-region>
        <span className="app-glyph" aria-hidden data-tauri-drag-region />
        <BoardSwitcher onOpen={onOpenBoard} />
      </div>
      <div className="titlebar-right">
        <IconButton label="Historial" active={drawerOpen} onClick={toggleDrawer}>
          <History size={16} strokeWidth={1.7} />
        </IconButton>
        <IconButton label="Ajustes" onClick={openSettings}>
          <SettingsIcon size={16} strokeWidth={1.7} />
        </IconButton>
        <span className="titlebar-sep" />
        <button type="button" className="win-btn" aria-label="Minimizar" onClick={() => void win.minimize()}>
          <Minus size={15} strokeWidth={1.4} />
        </button>
        <button type="button" className="win-btn" aria-label="Maximizar" onClick={() => void win.toggleMaximize()}>
          <Square size={12} strokeWidth={1.4} />
        </button>
        <button type="button" className="win-btn win-close" aria-label="Cerrar (sigue en la bandeja)" title="Cerrar (CanvasClip sigue en la bandeja)" onClick={() => void win.hide()}>
          <X size={16} strokeWidth={1.4} />
        </button>
      </div>
    </header>
  );
}
