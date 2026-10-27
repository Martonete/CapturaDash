import { useEffect, useRef } from "react";

import { useBoard } from "@/features/canvas/boardStore";
import { updateObjects } from "@/features/canvas/document";
import type { BoardObject } from "@/types/models";

interface Props {
  obj: BoardObject;
  editing: boolean;
}

/**
 * Nota adhesiva. Doble clic (o Enter) para editar; Escape, Ctrl+Enter o hacer
 * clic afuera termina. El texto se confirma una sola vez al terminar, así cada
 * edición es un único paso de deshacer.
 */
export function NoteView({ obj, editing }: Props) {
  const text = obj.props.text ?? "";
  const draft = useRef(text);
  const areaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!editing) return;
    draft.current = text;
    const el = areaRef.current;
    if (el) {
      el.focus();
      el.setSelectionRange(el.value.length, el.value.length);
    }
    return () => {
      // Al dejar de editar (por cualquier vía), confirmar si cambió.
      const value = draft.current;
      const st = useBoard.getState();
      const current = st.doc.objects[obj.id];
      if (current && value !== (current.props.text ?? "")) {
        st.commit((d) => updateObjects(d, { [obj.id]: { props: { ...current.props, text: value } } }));
      }
    };
    // Solo al entrar/salir de edición.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing, obj.id]);

  const stop = () => useBoard.getState().setEditing(null);

  return (
    <div className={`note note-${obj.props.color ?? "sand"}`}>
      {editing ? (
        <textarea
          ref={areaRef}
          className="note-text note-input"
          defaultValue={text}
          spellCheck={false}
          onChange={(e) => (draft.current = e.target.value)}
          onPointerDown={(e) => e.stopPropagation()}
          onBlur={stop}
          onKeyDown={(e) => {
            if (e.key === "Escape" || (e.key === "Enter" && (e.ctrlKey || e.metaKey))) {
              e.preventDefault();
              e.stopPropagation();
              stop();
            }
          }}
        />
      ) : (
        <div className="note-text">{text || <span className="note-placeholder">Doble clic para escribir</span>}</div>
      )}
    </div>
  );
}
