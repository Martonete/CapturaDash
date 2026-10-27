import { LayoutGrid, Pause, Settings as SettingsIcon, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { api } from "@/services/api";
import { on } from "@/services/events";
import { useHistory } from "@/stores/historyStore";
import { useSettings } from "@/stores/settingsStore";

import { CaptureList } from "../clipboard/CaptureList";
import { Flash } from "../common/Flash";
import { IconButton } from "../common/IconButton";

const CLOSE_MS = 140;

/** Ventana del panel lateral. Rust la muestra/oculta; acá se anima y se opera. */
export function PanelApp() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [phase, setPhase] = useState<"open" | "closing" | "hidden">("hidden");
  const closing = useRef<ReturnType<typeof setTimeout> | null>(null);
  const animate = (settings?.panelAnimations ?? true) && !settings?.reduceMotion;

  useEffect(() => {
    const offOpen = on("panel:opened", () => {
      if (closing.current) clearTimeout(closing.current);
      setPhase("open");
      // El historial se mantiene al día por eventos; al abrir solo se confirma.
      void useHistory.getState().reload();
    });
    const offClose = on("panel:close", () => {
      setPhase("closing");
      closing.current = setTimeout(() => {
        setPhase("hidden");
        void api.panelHide();
      }, animate ? CLOSE_MS : 0);
    });
    return () => {
      void offOpen.then((f) => f());
      void offClose.then((f) => f());
    };
  }, [animate]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented && !document.querySelector(".menu")) {
        e.preventDefault();
        void api.panelClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className={`panel panel-${phase} ${animate ? "" : "no-anim"}`}>
      <header className="panel-header" data-tauri-drag-region={false}>
        <div className="panel-title">
          <span className="panel-glyph" aria-hidden />
          <span>Historial</span>
          {settings && !settings.monitoring && (
            <button type="button" className="panel-paused" title="Reanudar captura automática" onClick={() => void update({ monitoring: true })}>
              <Pause size={10} strokeWidth={2.4} /> En pausa
            </button>
          )}
        </div>
        <div className="panel-tools">
          <IconButton label="Abrir pizarrón" onClick={() => void api.showMain("board")}>
            <LayoutGrid size={16} strokeWidth={1.7} />
          </IconButton>
          <IconButton label="Ajustes" onClick={() => void api.showMain("settings")}>
            <SettingsIcon size={16} strokeWidth={1.7} />
          </IconButton>
          <IconButton label="Cerrar (Esc)" onClick={() => void api.panelClose()}>
            <X size={16} strokeWidth={1.7} />
          </IconButton>
        </div>
      </header>
      <CaptureList keyboard emptyHint="Las capturas que hagas con Win + Shift + S, o cualquier imagen que copies, aparecerán acá." />
      <Flash />
    </div>
  );
}
