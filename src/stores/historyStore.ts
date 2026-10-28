import { create } from "zustand";

import { api, errorMessage } from "@/services/api";
import { on } from "@/services/events";
import type { Capture, CapturesChanged } from "@/types/models";
import { retry } from "@/utils/timing";

const PAGE = 60;

interface HistoryState {
  items: Capture[];
  favoritesOnly: boolean;
  loading: boolean;
  hasMore: boolean;
  error: string | null;
  reload: () => Promise<void>;
  loadMore: () => Promise<void>;
  setFavoritesOnly: (v: boolean) => void;
  apply: (e: CapturesChanged) => void;
}

/** Historial visible en esta ventana (panel o cajón del pizarrón). */
export const useHistory = create<HistoryState>((set, get) => ({
  items: [],
  favoritesOnly: false,
  loading: false,
  hasMore: true,
  error: null,

  reload: async () => {
    const fav = get().favoritesOnly;
    set({ loading: true });
    try {
      const items = await retry(() => api.listCaptures(fav, PAGE, 0), 4);
      if (get().favoritesOnly !== fav) return; // cambió el filtro mientras cargaba
      set({ items, hasMore: items.length === PAGE, loading: false, error: null });
    } catch (e) {
      set({ loading: false, error: errorMessage(e) });
    }
  },

  loadMore: async () => {
    const { loading, hasMore, items, favoritesOnly } = get();
    if (loading || !hasMore) return;
    set({ loading: true });
    try {
      const more = await api.listCaptures(favoritesOnly, PAGE, items.length);
      const known = new Set(get().items.map((c) => c.id));
      set((s) => ({
        items: [...s.items, ...more.filter((c) => !known.has(c.id))],
        hasMore: more.length === PAGE,
        loading: false,
      }));
    } catch (e) {
      set({ loading: false, error: errorMessage(e) });
    }
  },

  setFavoritesOnly: (v) => {
    if (v === get().favoritesOnly) return;
    set({ favoritesOnly: v, items: [], hasMore: true });
    void get().reload();
  },

  apply: (e) => {
    if (e.reset) {
      void get().reload();
      return;
    }
    set((s) => {
      let items = s.items;
      if (e.removed.length) {
        const gone = new Set(e.removed);
        items = items.filter((c) => !gone.has(c.id));
      }
      if (e.added && (!s.favoritesOnly || e.added.favorite) && !items.some((c) => c.id === e.added!.id)) {
        items = [e.added, ...items];
      }
      if (e.updated) {
        const u = e.updated;
        items = s.favoritesOnly && !u.favorite ? items.filter((c) => c.id !== u.id) : items.map((c) => (c.id === u.id ? u : c));
        if (s.favoritesOnly && u.favorite && !items.some((c) => c.id === u.id)) {
          items = [u, ...items].sort((a, b) => b.createdAt - a.createdAt);
        }
      }
      return { items };
    });
  },
}));

export function initHistory() {
  void on("captures:changed", (e) => useHistory.getState().apply(e));
  return useHistory.getState().reload();
}
