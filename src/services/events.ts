// Eventos emitidos por Rust (src-tauri/src/events.rs).

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { CapturesChanged, ObjectsInserted, Settings, ToastPayload } from "@/types/models";

export interface EventMap {
  "captures:changed": CapturesChanged;
  "settings:changed": Settings;
  "boards:changed": null;
  "board:objects-inserted": ObjectsInserted;
  "panel:opened": { focused: boolean };
  "panel:close": null;
  "toast:show": ToastPayload;
  "main:navigate": { view: "board" | "settings" };
  "app:flush": null;
}

export function on<K extends keyof EventMap>(event: K, handler: (payload: EventMap[K]) => void): Promise<UnlistenFn> {
  return listen<EventMap[K]>(event, (e) => handler(e.payload));
}
