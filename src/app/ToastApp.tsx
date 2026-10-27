import { AlertCircle, Check } from "lucide-react";
import { useEffect, useState } from "react";

import { api } from "@/services/api";
import { on } from "@/services/events";
import { thumbUrl } from "@/services/images";
import type { ToastPayload } from "@/types/models";
import { formatDims } from "@/utils/format";

const VISIBLE_MS = 2200;

/** Aviso discreto "Captura guardada". Rust lo posiciona y lo oculta. */
export function ToastApp() {
  const [toast, setToast] = useState<(ToastPayload & { seq: number }) | null>(null);
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    let seq = 0;
    let timer: ReturnType<typeof setTimeout>;
    const off = on("toast:show", (t) => {
      clearTimeout(timer);
      setLeaving(false);
      setToast({ ...t, seq: ++seq });
      timer = setTimeout(() => setLeaving(true), VISIBLE_MS);
    });
    return () => void off.then((f) => f());
  }, []);

  if (!toast) return null;
  const c = toast.capture;
  return (
    <button
      type="button"
      key={toast.seq}
      className={`toast toast-${toast.kind} ${leaving ? "is-leaving" : ""}`}
      onClick={() => void api.toastClicked()}
      title="Abrir historial"
    >
      {c ? (
        <img className="toast-thumb" src={thumbUrl(c.id)} alt="" />
      ) : (
        <span className="toast-icon">
          <AlertCircle size={18} strokeWidth={1.8} />
        </span>
      )}
      <span className="toast-text">
        <span className="toast-title">
          {c ? (
            <>
              <Check size={13} strokeWidth={2.4} className="toast-check" /> Captura guardada
            </>
          ) : toast.kind === "error" ? (
            "Atención"
          ) : (
            "CanvasClip"
          )}
        </span>
        <span className="toast-sub">{c ? `${formatDims(c.width, c.height)} · clic para ver el historial` : toast.message}</span>
      </span>
    </button>
  );
}
