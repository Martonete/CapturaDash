import { useEffect, useState } from "react";

interface Props {
  value: string;
  onChange: (accel: string) => Promise<string | null>;
}

const MODS: Record<string, string> = { Control: "Ctrl", Alt: "Alt", Shift: "Shift", Meta: "Super" };

/** Convierte un evento de teclado al formato de tauri-plugin-global-shortcut. */
function toAccelerator(e: KeyboardEvent): string | null {
  if (MODS[e.key]) return null; // solo modificadores: seguir esperando
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  const code = e.code;
  let key: string;
  if (code.startsWith("Key")) key = code.slice(3);
  else if (code.startsWith("Digit")) key = code.slice(5);
  else key = code; // Space, F1…F24, ArrowUp, Backquote, etc.
  parts.push(key);
  return parts.join("+");
}

function pretty(accel: string) {
  return accel.split("+").map((p) => (p === "Space" ? "Espacio" : p));
}

export function ShortcutRecorder({ value, onChange }: Props) {
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!recording) return;
    const onKey = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      const accel = toAccelerator(e);
      if (!accel) return;
      setRecording(false);
      setError(await onChange(accel));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <div className="shortcut">
      <button type="button" className={`shortcut-field ${recording ? "is-recording" : ""}`} onClick={() => { setError(null); setRecording((r) => !r); }}>
        {recording ? (
          <span className="shortcut-hint">Presioná la combinación…</span>
        ) : (
          pretty(value).map((k, i) => <kbd key={i}>{k}</kbd>)
        )}
      </button>
      {error && <p className="setting-error">{error}</p>}
    </div>
  );
}
