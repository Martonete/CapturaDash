import { Check, ChevronDown, Copy, Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { flushSave } from "@/features/canvas/persistence";
import { api, errorMessage } from "@/services/api";
import { thumbUrl } from "@/services/images";
import { useBoards } from "@/stores/boardsStore";
import { flash } from "@/stores/flashStore";
import type { BoardSummary } from "@/types/models";

import { IconButton } from "../common/IconButton";

/** Selector compacto de pizarrones con miniaturas, en la barra de título. */
export function BoardSwitcher({ onOpen }: { onOpen: (id: string) => void }) {
  const boards = useBoards((s) => s.boards);
  const activeId = useBoards((s) => s.activeId);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const active = boards.find((b) => b.id === activeId);

  useEffect(() => {
    if (!open) return;
    void useBoards.getState().refresh();
    const onDown = (e: PointerEvent) => !ref.current?.contains(e.target as Node) && setOpen(false);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const choose = (id: string) => {
    setOpen(false);
    if (id !== activeId) onOpen(id);
  };

  return (
    <div className="board-switcher" ref={ref}>
      <button type="button" className={`board-trigger ${open ? "is-open" : ""}`} onClick={() => setOpen((v) => !v)} aria-haspopup="dialog" aria-expanded={open}>
        <span className="board-trigger-name">{active?.name ?? "…"}</span>
        <ChevronDown size={14} strokeWidth={1.8} />
      </button>
      {open && (
        <div className="popover boards-popover" role="dialog" aria-label="Pizarrones">
          <div className="boards-list">
            {boards.map((b) => (
              <BoardRow key={b.id} board={b} active={b.id === activeId} onChoose={choose} canDelete={boards.length > 1} />
            ))}
          </div>
          <NewBoard onCreated={choose} />
        </div>
      )}
    </div>
  );
}

function BoardRow({ board, active, onChoose, canDelete }: { board: BoardSummary; active: boolean; onChoose: (id: string) => void; canDelete: boolean }) {
  const [renaming, setRenaming] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [name, setName] = useState(board.name);

  const rename = async () => {
    setRenaming(false);
    if (name.trim() && name.trim() !== board.name) {
      try {
        await api.renameBoard(board.id, name);
      } catch (e) {
        flash(errorMessage(e), "error");
        setName(board.name);
      }
    } else setName(board.name);
  };

  const remove = async () => {
    try {
      await flushSave();
      const next = await api.deleteBoard(board.id);
      if (active) onChoose(next);
    } catch (e) {
      flash(errorMessage(e), "error");
    }
  };

  if (confirm) {
    return (
      <div className="board-row is-confirm">
        <span>
          ¿Eliminar “{board.name}”? Las capturas seguirán en el historial.
        </span>
        <button type="button" className="btn btn-ghost" onClick={() => setConfirm(false)}>Cancelar</button>
        <button type="button" className="btn btn-danger" onClick={remove}>Eliminar</button>
      </div>
    );
  }

  return (
    <div className={`board-row ${active ? "is-active" : ""}`} onClick={() => !renaming && onChoose(board.id)}>
      <BoardPreview board={board} />
      <div className="board-info">
        {renaming ? (
          <input
            className="board-rename"
            value={name}
            autoFocus
            maxLength={80}
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => setName(e.target.value)}
            onBlur={rename}
            onKeyDown={(e) => {
              if (e.key === "Enter") void rename();
              if (e.key === "Escape") {
                e.stopPropagation();
                setName(board.name);
                setRenaming(false);
              }
            }}
          />
        ) : (
          <span className="board-name">{board.name}</span>
        )}
        <span className="board-count">{board.objectCount === 1 ? "1 elemento" : `${board.objectCount} elementos`}</span>
      </div>
      {active && !renaming && <Check size={14} className="board-check" strokeWidth={2} />}
      <div className="board-actions" onClick={(e) => e.stopPropagation()}>
        <IconButton label="Renombrar" size="s" onClick={() => setRenaming(true)}>
          <Pencil size={13} strokeWidth={1.8} />
        </IconButton>
        <IconButton
          label="Duplicar"
          size="s"
          onClick={async () => {
            try {
              await flushSave();
              await api.duplicateBoard(board.id);
            } catch (e) {
              flash(errorMessage(e), "error");
            }
          }}
        >
          <Copy size={13} strokeWidth={1.8} />
        </IconButton>
        <IconButton label="Eliminar" size="s" tone="danger" disabled={!canDelete} onClick={() => setConfirm(true)}>
          <Trash2 size={13} strokeWidth={1.8} />
        </IconButton>
      </div>
    </div>
  );
}

/** Miniatura representativa: la disposición real del pizarrón, a escala. */
function BoardPreview({ board }: { board: BoardSummary }) {
  const W = 56;
  const H = 38;
  const items = board.preview;
  if (!items.length) return <div className="board-preview is-empty" />;
  const x1 = Math.min(...items.map((i) => i.x));
  const y1 = Math.min(...items.map((i) => i.y));
  const x2 = Math.max(...items.map((i) => i.x + i.width));
  const y2 = Math.max(...items.map((i) => i.y + i.height));
  const s = Math.min((W - 8) / Math.max(1, x2 - x1), (H - 8) / Math.max(1, y2 - y1));
  const ox = (W - (x2 - x1) * s) / 2;
  const oy = (H - (y2 - y1) * s) / 2;
  return (
    <div className="board-preview">
      {items.map((i, k) => {
        const style = { left: ox + (i.x - x1) * s, top: oy + (i.y - y1) * s, width: Math.max(2, i.width * s), height: Math.max(2, i.height * s) };
        return i.kind === "image" && i.captureId ? (
          <img key={k} src={thumbUrl(i.captureId)} alt="" style={style} loading="lazy" />
        ) : (
          <span key={k} className={`note-${i.color ?? "sand"}`} style={style} />
        );
      })}
    </div>
  );
}

function NewBoard({ onCreated }: { onCreated: (id: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState("");
  const create = async () => {
    const n = name.trim();
    setEditing(false);
    setName("");
    if (!n) return;
    try {
      const b = await api.createBoard(n);
      onCreated(b.id);
    } catch (e) {
      flash(errorMessage(e), "error");
    }
  };
  return editing ? (
    <div className="board-new is-editing">
      <input
        autoFocus
        placeholder="Nombre del pizarrón"
        value={name}
        maxLength={80}
        onChange={(e) => setName(e.target.value)}
        onBlur={create}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
          if (e.key === "Escape") {
            e.stopPropagation();
            setName("");
            setEditing(false);
          }
        }}
      />
    </div>
  ) : (
    <button type="button" className="board-new" onClick={() => setEditing(true)}>
      <Plus size={14} strokeWidth={1.8} /> Nuevo pizarrón
    </button>
  );
}
