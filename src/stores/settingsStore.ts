import { create } from "zustand";

import { api, errorMessage } from "@/services/api";
import { on } from "@/services/events";
import type { Settings } from "@/types/models";

interface SettingsState {
  settings: Settings | null;
  error: string | null;
  load: () => Promise<void>;
  /** Aplica un cambio parcial. Devuelve el mensaje de error, si lo hubo. */
  update: (patch: Partial<Settings>) => Promise<string | null>;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: null,
  error: null,
  load: async () => {
    try {
      set({ settings: await api.getSettings(), error: null });
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },
  update: async (patch) => {
    const current = get().settings;
    if (!current) return "La configuración todavía no cargó";
    try {
      const saved = await api.updateSettings({ ...current, ...patch });
      set({ settings: saved, error: null });
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  },
}));

/** Refleja la apariencia en <html> (todas las ventanas la comparten). */
function applyAppearance(s: Settings) {
  const root = document.documentElement;
  root.dataset.density = s.density;
  root.dataset.effects = s.effects;
  root.dataset.reduceMotion = String(s.reduceMotion);
}

export function initSettings() {
  useSettings.subscribe((state) => {
    if (state.settings) applyAppearance(state.settings);
  });
  void on("settings:changed", (s) => useSettings.setState({ settings: s }));
  return useSettings.getState().load();
}
