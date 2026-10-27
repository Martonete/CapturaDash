import { Check, Copy, MoreHorizontal, Plus, Star, Trash2 } from "lucide-react";
import { memo, useState, type DragEvent, type MouseEvent } from "react";

import { CAPTURE_DRAG_TYPE, thumbUrl } from "@/services/images";
import type { Capture } from "@/types/models";
import { formatDims, relativeTime } from "@/utils/format";

import { IconButton } from "../common/IconButton";

const MAX_THUMB_H = 200;

interface Props {
  capture: Capture;
  selected: boolean;
  confirmingDelete: boolean;
  onFocusCard: (id: string) => void;
  onCopy: (c: Capture) => Promise<boolean>;
  onAdd: (c: Capture) => void;
  onMenu: (c: Capture, x: number, y: number) => void;
  onConfirmDelete: (c: Capture | null) => void;
  onDelete: (c: Capture) => void;
  now: number;
}

/** Tarjeta de una captura: miniatura protagonista, acciones al pasar el cursor. */
export const CaptureCard = memo(function CaptureCard(p: Props) {
  const c = p.capture;
  const [copied, setCopied] = useState(false);
  const ratio = c.thumbHeight / Math.max(1, c.thumbWidth);
  const tall = ratio * 320 > MAX_THUMB_H;

  const copy = async () => {
    if (await p.onCopy(c)) {
      setCopied(true);
      setTimeout(() => setCopied(false), 900);
    }
  };

  const onDragStart = (e: DragEvent) => {
    e.dataTransfer.setData(CAPTURE_DRAG_TYPE, c.id);
    e.dataTransfer.effectAllowed = "copy";
    const img = (e.currentTarget as HTMLElement).querySelector("img");
    if (img) e.dataTransfer.setDragImage(img, img.clientWidth / 2, Math.min(img.clientHeight / 2, 60));
  };

  const onContext = (e: MouseEvent) => {
    e.preventDefault();
    p.onMenu(c, e.clientX, e.clientY);
  };

  return (
    <article
      className={`capture ${p.selected ? "is-selected" : ""}`}
      data-capture-id={c.id}
      tabIndex={-1}
      draggable
      onDragStart={onDragStart}
      onDoubleClick={copy}
      onPointerDown={() => p.onFocusCard(c.id)}
      onContextMenu={onContext}
      aria-label={`Captura ${formatDims(c.width, c.height)}, ${relativeTime(c.createdAt, p.now)}`}
    >
      <div className="capture-thumb" style={{ aspectRatio: tall ? undefined : `${c.thumbWidth} / ${c.thumbHeight}`, height: tall ? MAX_THUMB_H : undefined }}>
        <img src={thumbUrl(c.id)} alt="" loading="lazy" decoding="async" className={tall ? "is-tall" : ""} draggable={false} />
        <div className="capture-actions">
          <IconButton label="Copiar (doble clic)" size="s" onClick={copy}>
            <Copy size={14} strokeWidth={1.8} />
          </IconButton>
          <IconButton label="Agregar al pizarrón actual" size="s" onClick={() => p.onAdd(c)}>
            <Plus size={15} strokeWidth={1.8} />
          </IconButton>
          <IconButton label="Más acciones" size="s" onClick={(e) => { const r = e.currentTarget.getBoundingClientRect(); p.onMenu(c, r.left, r.bottom + 4); }}>
            <MoreHorizontal size={15} strokeWidth={1.8} />
          </IconButton>
        </div>
        {copied && (
          <div className="capture-copied">
            <Check size={16} strokeWidth={2.2} /> Copiada
          </div>
        )}
      </div>
      {p.confirmingDelete ? (
        <div className="capture-confirm">
          <span>{c.usageCount > 0 ? `Se usa en ${c.usageCount} pizarrón${c.usageCount > 1 ? "es" : ""}. ¿Eliminar en todos?` : "¿Eliminar definitivamente?"}</span>
          <button type="button" className="btn btn-ghost" onClick={() => p.onConfirmDelete(null)}>Cancelar</button>
          <button type="button" className="btn btn-danger" autoFocus onClick={() => p.onDelete(c)}>
            <Trash2 size={13} strokeWidth={1.8} /> Eliminar
          </button>
        </div>
      ) : (
        <div className="capture-meta">
          {c.favorite && <Star size={12} className="capture-fav" fill="currentColor" strokeWidth={0} aria-label="Favorita" />}
          <span>{relativeTime(c.createdAt, p.now)}</span>
          <span className="capture-dot">·</span>
          <span>{formatDims(c.width, c.height)}</span>
          {c.usageCount > 0 && <span className="capture-badge" title="Pizarrones que la usan">{c.usageCount === 1 ? "En 1 pizarrón" : `En ${c.usageCount} pizarrones`}</span>}
        </div>
      )}
    </article>
  );
});
