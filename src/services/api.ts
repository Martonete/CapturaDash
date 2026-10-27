// Envoltorios tipados de los comandos de Rust (src-tauri/src/commands).

import { invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  Board,
  BoardChanges,
  BoardDocument,
  BoardObject,
  BoardSummary,
  Capture,
  PasteResult,
  SaveReport,
  Settings,
  StorageInfo,
} from "@/types/models";

export const api = {
  // Historial
  listCaptures: (favorites: boolean, limit: number, offset: number) =>
    invoke<Capture[]>("list_captures", { favorites, limit, offset }),
  getCaptures: (ids: string[]) => invoke<Capture[]>("get_captures", { ids }),
  setFavorite: (id: string, favorite: boolean) => invoke<Capture>("set_favorite", { id, favorite }),
  deleteCapture: (id: string, force: boolean) => invoke<void>("delete_capture", { id, force }),
  copyCapture: (id: string) => invoke<void>("copy_capture", { id }),
  pasteClipboard: () => invoke<PasteResult>("paste_clipboard"),
  importImageBytes: (bytes: Uint8Array) => invoke<Capture>("import_image_bytes", bytes),
  clearHistory: () => invoke<number>("clear_history"),
  storageInfo: () => invoke<StorageInfo>("storage_info"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),

  // Pizarrones
  listBoards: () => invoke<BoardSummary[]>("list_boards"),
  createBoard: (name: string) => invoke<Board>("create_board", { name }),
  renameBoard: (id: string, name: string) => invoke<Board>("rename_board", { id, name }),
  duplicateBoard: (id: string) => invoke<Board>("duplicate_board", { id }),
  deleteBoard: (id: string) => invoke<string>("delete_board", { id }),
  loadBoard: (id: string) => invoke<BoardDocument>("load_board", { id }),
  saveBoardChanges: (changes: BoardChanges) => invoke<SaveReport>("save_board_changes", { changes }),
  saveCamera: (id: string, x: number, y: number, zoom: number) => invoke<void>("save_camera", { id, x, y, zoom }),
  getActiveBoard: () => invoke<string>("get_active_board"),
  setActiveBoard: (id: string) => invoke<void>("set_active_board", { id }),
  addCaptureToBoard: (captureId: string, boardId?: string) =>
    invoke<{ boardId: string; boardName: string }>("add_capture_to_board", { captureId, boardId: boardId ?? null }),

  // Configuración
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  appInfo: () => invoke<AppInfo>("app_info"),
  changeDataDir: (path: string) => invoke<void>("change_data_dir", { path }),
  openDataDir: () => invoke<void>("open_data_dir"),

  // Ventanas
  panelHide: () => invoke<void>("panel_hide"),
  panelOpen: () => invoke<void>("panel_open"),
  panelClose: () => invoke<void>("panel_close"),
  showMain: (view?: "board" | "settings") => invoke<void>("show_main", { view: view ?? null }),
  toastClicked: () => invoke<void>("toast_clicked"),
  toastHide: () => invoke<void>("toast_hide"),
};

export type { BoardObject };

/** Mensaje legible de un error de invoke (Rust devuelve texto). */
export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "Ocurrió un error inesperado";
}
