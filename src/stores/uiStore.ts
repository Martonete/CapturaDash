import { create } from "zustand";

const DRAWER_KEY = "canvasclip.drawerOpen";

function readDrawer(): boolean {
  try {
    return localStorage.getItem(DRAWER_KEY) === "1";
  } catch {
    return false;
  }
}

/** Estado de interfaz de la ventana principal (no persistente salvo el cajón). */
interface UiState {
  settingsOpen: boolean;
  drawerOpen: boolean;
  setSettingsOpen: (v: boolean) => void;
  toggleDrawer: () => void;
}

export const useUi = create<UiState>((set, get) => ({
  settingsOpen: false,
  drawerOpen: readDrawer(),
  setSettingsOpen: (v) => set({ settingsOpen: v }),
  toggleDrawer: () => {
    const v = !get().drawerOpen;
    try {
      localStorage.setItem(DRAWER_KEY, v ? "1" : "0");
    } catch {
      /* almacenamiento no disponible: solo se pierde la preferencia */
    }
    set({ drawerOpen: v });
  },
}));
