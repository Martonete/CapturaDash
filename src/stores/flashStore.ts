import { create } from "zustand";

/** Mensaje breve dentro de la ventana ("Copiado", "Agregado a General"). */
interface FlashState {
  message: string | null;
  tone: "info" | "error";
  seq: number;
  show: (message: string, tone?: "info" | "error") => void;
}

let timer: ReturnType<typeof setTimeout> | null = null;

export const useFlash = create<FlashState>((set) => ({
  message: null,
  tone: "info",
  seq: 0,
  show: (message, tone = "info") => {
    if (timer) clearTimeout(timer);
    set((s) => ({ message, tone, seq: s.seq + 1 }));
    timer = setTimeout(() => set({ message: null }), tone === "error" ? 4200 : 1800);
  },
}));

export const flash = (message: string, tone: "info" | "error" = "info") => useFlash.getState().show(message, tone);
