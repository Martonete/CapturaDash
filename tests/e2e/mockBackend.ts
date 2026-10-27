// Backend simulado SOLO para QA visual en Chromium (no forma parte de la app).
// Implementa en memoria los comandos que usa la interfaz, con la API oficial
// de mocks de Tauri. Las imágenes las sirve Playwright (http://ccimg.localhost).

import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import type { BoardObject, Capture, Group, Settings } from "@/types/models";

type Board = { id: string; name: string; createdAt: number; updatedAt: number; cameraX: number; cameraY: number; zoom: number };

const now = Date.now();
const settings: Settings = {
  launchAtStartup: true, startMinimized: true, monitoring: true, shortcut: "Alt+Shift+V", edgeEnabled: true,
  edgeDelayMs: 300, panelAnimations: true, closeBehavior: "auto", historyLimit: 500, notifications: true,
  density: "comfortable", effects: "subtle", reduceMotion: false, grid: "dots",
};
const captures: Capture[] = [];
const boards: Board[] = [{ id: "b1", name: "General", createdAt: now, updatedAt: now, cameraX: 0, cameraY: 0, zoom: 1 }];
const objects = new Map<string, BoardObject>();
const groups = new Map<string, Group>();
let active = "b1";

function usage(id: string) {
  return new Set([...objects.values()].filter((o) => o.captureId === id).map((o) => o.boardId)).size;
}
const withUsage = (c: Capture) => ({ ...c, usageCount: usage(c.id) });

declare global {
  interface Window {
    __cc: { addCapture: (id: string, w: number, h: number, minutesAgo?: number) => void; captures: Capture[] };
  }
}

window.__cc = {
  captures,
  addCapture(id, w, h, minutesAgo = 0) {
    const c: Capture = {
      id, hash: id, mime: "image/png", width: w, height: h,
      thumbWidth: Math.min(640, w), thumbHeight: Math.round((Math.min(640, w) / w) * h),
      byteSize: w * h, source: "clipboard", favorite: false, createdAt: Date.now() - minutesAgo * 60_000, usageCount: 0,
    };
    captures.unshift(c);
    captures.sort((a, b) => b.createdAt - a.createdAt);
    void emit("captures:changed", { added: c, removed: [], reset: false });
  },
};

const handlers: Record<string, (a: any) => unknown> = {
  list_captures: ({ favorites, limit, offset }) =>
    captures.filter((c) => !favorites || c.favorite).slice(offset, offset + limit).map(withUsage),
  get_captures: ({ ids }) => captures.filter((c) => ids.includes(c.id)).map(withUsage),
  set_favorite: ({ id, favorite }) => {
    const c = captures.find((x) => x.id === id)!;
    c.favorite = favorite;
    void emit("captures:changed", { updated: withUsage(c), removed: [], reset: false });
    return withUsage(c);
  },
  delete_capture: ({ id }) => {
    captures.splice(captures.findIndex((c) => c.id === id), 1);
    void emit("captures:changed", { removed: [id], reset: false });
  },
  copy_capture: () => null,
  copy_text: () => null,
  paste_clipboard: () => ({ kind: "Empty" }),
  storage_info: () => ({ dataDir: "C:\\Users\\marti\\AppData\\Local\\com.canvasclip.app", captureCount: captures.length, protectedCount: 2, bytes: 48_300_000 }),
  app_info: () => ({ version: "0.1.0", dataDir: "C:\\Users\\marti\\AppData\\Local\\com.canvasclip.app", defaultDataDir: "", platform: "windows" }),
  list_boards: () =>
    boards.map((b) => {
      const objs = [...objects.values()].filter((o) => o.boardId === b.id);
      return { ...b, objectCount: objs.length, preview: objs.map((o) => ({ kind: o.kind, x: o.x, y: o.y, width: o.width, height: o.height, captureId: o.captureId, color: o.props.color ?? null })) };
    }),
  get_active_board: () => active,
  set_active_board: ({ id }) => void (active = id),
  create_board: ({ name }) => {
    const b = { id: `b${boards.length + 1}`, name, createdAt: Date.now(), updatedAt: Date.now(), cameraX: 0, cameraY: 0, zoom: 1 };
    boards.push(b);
    void emit("boards:changed", null);
    return b;
  },
  rename_board: ({ id, name }) => {
    const b = boards.find((x) => x.id === id)!;
    b.name = name;
    void emit("boards:changed", null);
    return b;
  },
  load_board: ({ id }) => ({
    board: boards.find((b) => b.id === id),
    objects: [...objects.values()].filter((o) => o.boardId === id),
    groups: [...groups.values()].filter((g) => g.boardId === id),
  }),
  save_board_changes: ({ changes }) => {
    for (const g of changes.upsertGroups) groups.set(g.id, g);
    for (const o of changes.upsertObjects) objects.set(o.id, o);
    for (const id of changes.deleteObjects) objects.delete(id);
    for (const id of changes.deleteGroups) groups.delete(id);
    return { skippedObjects: [] };
  },
  save_camera: ({ id, x, y, zoom }) => Object.assign(boards.find((b) => b.id === id)!, { cameraX: x, cameraY: y, zoom }),
  get_settings: () => settings,
  update_settings: ({ settings: s }) => {
    Object.assign(settings, s);
    void emit("settings:changed", settings);
    return settings;
  },
  add_capture_to_board: () => ({ boardId: active, boardName: "General" }),
  panel_hide: () => null,
  panel_close: () => void emit("panel:close", null),
  show_main: () => null,
};

export function installMocks(label: string) {
  mockWindows(label);
  mockIPC(
    (cmd, args) => {
      const h = handlers[cmd];
      if (!h) {
        if (cmd.startsWith("plugin:")) return null;
        throw `comando no simulado: ${cmd}`;
      }
      return h(args);
    },
    { shouldMockEvents: true },
  );
}
