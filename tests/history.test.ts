import { describe, expect, it } from "vitest";

import { emptyUndo, HISTORY_LIMIT, record, redo, undo } from "@/features/canvas/history";

describe("deshacer / rehacer", () => {
  it("recorre estados en ambos sentidos", () => {
    let h = emptyUndo<number>();
    let cur = 0;
    for (const next of [1, 2, 3]) {
      h = record(h, cur);
      cur = next;
    }
    let r = undo(h, cur)!;
    expect(r.state).toBe(2);
    r = undo(r.history, r.state)!;
    expect(r.state).toBe(1);
    const fwd = redo(r.history, r.state)!;
    expect(fwd.state).toBe(2);
    expect(redo(emptyUndo<number>(), 0)).toBeNull();
    expect(undo(emptyUndo<number>(), 0)).toBeNull();
  });

  it("un cambio nuevo descarta lo que se podía rehacer", () => {
    let h = record(emptyUndo<number>(), 0);
    const r = undo(h, 1)!;
    h = record(r.history, r.state);
    expect(h.future).toEqual([]);
  });

  it("tiene un límite de pasos", () => {
    let h = emptyUndo<number>();
    for (let i = 0; i < HISTORY_LIMIT + 50; i++) h = record(h, i);
    expect(h.past.length).toBe(HISTORY_LIMIT);
    expect(h.past[0]).toBe(50);
  });
});
