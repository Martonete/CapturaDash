import { useBoard } from "@/features/canvas/boardStore";
import { membersOf, rectOf } from "@/features/canvas/document";
import { boundsOf, type Rect } from "@/features/canvas/geometry";

const HANDLES = ["nw", "ne", "sw", "se"] as const;

function frame(r: Rect, pad = 0) {
  return { left: r.x - pad, top: r.y - pad, width: r.width + pad * 2, height: r.height + pad * 2 };
}

/**
 * Selección dibujada dentro del mundo; los grosores se compensan con --zoom
 * para verse siempre iguales en pantalla.
 */
export function SelectionLayer({ hideHandles }: { hideHandles: boolean }) {
  const selection = useBoard((s) => s.selection);
  const doc = useBoard((s) => s.doc);
  const isolated = useBoard((s) => s.isolatedGroup);
  const editing = useBoard((s) => s.editingId);

  const objs = selection.map((id) => doc.objects[id]).filter(Boolean);
  const box = boundsOf(objs.map(rectOf));

  // Grupos completamente seleccionados: marco punteado elegante.
  const groupIds = [...new Set(objs.map((o) => o.groupId).filter((g): g is string => !!g && g !== isolated))];
  const groupFrames = groupIds
    .map((g) => boundsOf(membersOf(doc, g).map((id) => rectOf(doc.objects[id]))))
    .filter((r): r is Rect => !!r);
  const isolatedFrame = isolated ? boundsOf(membersOf(doc, isolated).map((id) => rectOf(doc.objects[id]))) : null;

  return (
    <div className="selection-layer" aria-hidden>
      {isolatedFrame && <div className="sel-group is-isolated" style={frame(isolatedFrame, 10)} />}
      {groupFrames.map((r, i) => (
        <div key={i} className="sel-group" style={frame(r, 8)} />
      ))}
      {objs.length > 1 && objs.map((o) => <div key={o.id} className="sel-item" style={frame(rectOf(o))} />)}
      {box && !editing && (
        <div className="sel-box" style={frame(box)}>
          {!hideHandles &&
            HANDLES.map((h) => <div key={h} className={`sel-handle sel-${h}`} data-handle={h} />)}
        </div>
      )}
    </div>
  );
}
