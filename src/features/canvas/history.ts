// Deshacer/rehacer por instantáneas. Los documentos son inmutables y
// comparten estructura, así que guardar cada estado cuesta poco.

export const HISTORY_LIMIT = 200;

export interface UndoState<T> {
  past: T[];
  future: T[];
}

export const emptyUndo = <T>(): UndoState<T> => ({ past: [], future: [] });

/** Registra que `before` fue reemplazado por un estado nuevo. */
export function record<T>(h: UndoState<T>, before: T): UndoState<T> {
  const past = h.past.length >= HISTORY_LIMIT ? h.past.slice(1) : h.past.slice();
  past.push(before);
  return { past, future: [] };
}

export function undo<T>(h: UndoState<T>, current: T): { state: T; history: UndoState<T> } | null {
  if (h.past.length === 0) return null;
  const past = h.past.slice(0, -1);
  return { state: h.past[h.past.length - 1], history: { past, future: [current, ...h.future] } };
}

export function redo<T>(h: UndoState<T>, current: T): { state: T; history: UndoState<T> } | null {
  if (h.future.length === 0) return null;
  const [next, ...future] = h.future;
  return { state: next, history: { past: [...h.past, current], future } };
}
