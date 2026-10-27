import { Copy, Group, Trash2, Ungroup } from "lucide-react";

import { colorSelection, copySelection, deleteSelection, groupSelection, ungroupSelection } from "@/features/canvas/actions";
import { useBoard } from "@/features/canvas/boardStore";
import { rectOf } from "@/features/canvas/document";
import { boundsOf } from "@/features/canvas/geometry";
import { NOTE_COLORS } from "@/types/models";

import { IconButton } from "../common/IconButton";

const COLOR_LABELS: Record<string, string> = {
  sand: "Arena",
  sage: "Salvia",
  sky: "Cielo",
  rose: "Rosa",
  lilac: "Lila",
  graphite: "Grafito",
};

/** Barra flotante sobre la selección: solo las acciones que aplican. */
export function ContextToolbar() {
  const selection = useBoard((s) => s.selection);
  const doc = useBoard((s) => s.doc);
  const editing = useBoard((s) => s.editingId);
  const objs = selection.map((id) => doc.objects[id]).filter(Boolean);
  const box = boundsOf(objs.map(rectOf));
  if (!box || editing) return null;

  const notes = objs.filter((o) => o.kind === "note");
  const grouped = objs.some((o) => o.groupId);
  const oneGroup = grouped && new Set(objs.map((o) => o.groupId)).size === 1 && objs.every((o) => o.groupId);
  const currentColor = notes.length && notes.every((n) => n.props.color === notes[0].props.color) ? notes[0].props.color ?? "sand" : null;

  return (
    <div className="ctx-toolbar" data-ui style={{ left: box.x + box.width / 2, top: box.y }}>
      {notes.length > 0 && (
        <div className="ctx-colors" role="radiogroup" aria-label="Color de la nota">
          {NOTE_COLORS.map((c) => (
            <button
              key={c}
              type="button"
              role="radio"
              aria-checked={currentColor === c}
              title={COLOR_LABELS[c]}
              className={`ctx-color note-${c} ${currentColor === c ? "is-current" : ""}`}
              onClick={() => colorSelection(c)}
            />
          ))}
        </div>
      )}
      {notes.length > 0 && <span className="ctx-sep" />}
      <IconButton label="Copiar (Ctrl+C)" size="s" onClick={() => void copySelection()}>
        <Copy size={14} strokeWidth={1.8} />
      </IconButton>
      {objs.length > 1 && !oneGroup && (
        <IconButton label="Agrupar (Ctrl+G)" size="s" onClick={groupSelection}>
          <Group size={15} strokeWidth={1.7} />
        </IconButton>
      )}
      {grouped && (
        <IconButton label="Desagrupar (Ctrl+Shift+G)" size="s" onClick={ungroupSelection}>
          <Ungroup size={15} strokeWidth={1.7} />
        </IconButton>
      )}
      <IconButton label="Quitar del pizarrón (Supr)" size="s" tone="danger" onClick={deleteSelection}>
        <Trash2 size={14} strokeWidth={1.8} />
      </IconButton>
    </div>
  );
}
