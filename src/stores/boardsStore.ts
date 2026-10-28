import { create } from "zustand";

import { api, errorMessage } from "@/services/api";
import { on } from "@/services/events";
import { retry } from "@/utils/timing";
import type { BoardSummary } from "@/types/models";

interface BoardsState {
  boards: BoardSummary[];
  activeId: string | null;
  error: string | null;
  refresh: () => Promise<void>;
  setActive: (id: string) => Promise<void>;
}

/** Biblioteca de pizarrones y cuál está activo. */
export const useBoards = create<BoardsState>((set) => ({
  boards: [],
  activeId: null,
  error: null,
  refresh: async () => {
    try {
      const [boards, activeId] = await retry(() => Promise.all([api.listBoards(), api.getActiveBoard()]));
      set({ boards, activeId, error: null });
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },
  setActive: async (id) => {
    await api.setActiveBoard(id);
    set({ activeId: id });
  },
}));

export function initBoards() {
  void on("boards:changed", () => void useBoards.getState().refresh());
  return useBoards.getState().refresh();
}
