import { useFlash } from "@/stores/flashStore";

/** Mensaje breve flotante dentro de la ventana. */
export function Flash() {
  const { message, tone, seq } = useFlash();
  if (!message) return null;
  return (
    <div key={seq} className={`flash flash-${tone}`} role="status" aria-live="polite">
      {message}
    </div>
  );
}
