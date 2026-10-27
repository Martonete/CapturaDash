import { Maximize, Minus, Plus, StickyNote } from "lucide-react";

import { addNote, fitAll, resetZoom, zoomBy } from "@/features/canvas/actions";
import { useCameraSnapshot } from "@/features/canvas/camera";

import { IconButton } from "../common/IconButton";

/** Controles mínimos en la parte inferior: nota, zoom y encuadre. */
export function CanvasHud() {
  const cam = useCameraSnapshot(80);
  return (
    <div className="hud" data-ui>
      <IconButton label="Nueva nota (doble clic en el fondo)" onClick={() => addNote()}>
        <StickyNote size={16} strokeWidth={1.7} />
      </IconButton>
      <span className="hud-sep" />
      <IconButton label="Alejar (Ctrl −)" onClick={() => zoomBy(0.8)}>
        <Minus size={15} strokeWidth={1.8} />
      </IconButton>
      <button type="button" className="hud-zoom" title="Restablecer al 100 % (Ctrl 0)" onClick={resetZoom}>
        {Math.round(cam.zoom * 100)}%
      </button>
      <IconButton label="Acercar (Ctrl +)" onClick={() => zoomBy(1.25)}>
        <Plus size={15} strokeWidth={1.8} />
      </IconButton>
      <span className="hud-sep" />
      <IconButton label="Encuadrar todo (F)" onClick={() => fitAll()}>
        <Maximize size={15} strokeWidth={1.7} />
      </IconButton>
    </div>
  );
}
