/** Ejecuta `fn` como máximo una vez cada `ms`, garantizando la última llamada. */
export function throttle<A extends unknown[]>(fn: (...args: A) => void, ms: number) {
  let last = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: A | null = null;
  const run = () => {
    last = performance.now();
    timer = null;
    if (pending) {
      const args = pending;
      pending = null;
      fn(...args);
    }
  };
  return (...args: A) => {
    pending = args;
    const wait = ms - (performance.now() - last);
    if (wait <= 0) run();
    else if (!timer) timer = setTimeout(run, wait);
  };
}

export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: A | null = null;
  const debounced = (...args: A) => {
    pending = args;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      const a = pending!;
      pending = null;
      fn(...a);
    }, ms);
  };
  /** Ejecuta ya la llamada pendiente, si la hay. */
  debounced.flush = () => {
    if (timer) {
      clearTimeout(timer);
      timer = null;
      const a = pending!;
      pending = null;
      fn(...a);
    }
  };
  return debounced;
}

export function isEditableTarget(t: EventTarget | null): boolean {
  if (!(t instanceof HTMLElement)) return false;
  return t.isContentEditable || t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT";
}

/** Reintenta una operación de arranque unas veces antes de rendirse. */
export async function retry<T>(fn: () => Promise<T>, attempts = 6, delayMs = 300): Promise<T> {
  let last: unknown;
  for (let i = 0; i < attempts; i++) {
    try {
      return await fn();
    } catch (e) {
      last = e;
      await new Promise((r) => setTimeout(r, delayMs * (i + 1)));
    }
  }
  throw last;
}
