// Espejo de los tipos de Rust (src-tauri/core/src/models.rs), en camelCase.

export interface Capture {
  id: string;
  hash: string;
  mime: string;
  width: number;
  height: number;
  thumbWidth: number;
  thumbHeight: number;
  byteSize: number;
  source: "clipboard" | "paste" | string;
  favorite: boolean;
  createdAt: number;
  /** Cantidad de pizarrones que usan esta captura. */
  usageCount: number;
}

export interface Board {
  id: string;
  name: string;
  createdAt: number;
  updatedAt: number;
  cameraX: number;
  cameraY: number;
  zoom: number;
}

export interface PreviewItem {
  kind: ObjectKind;
  x: number;
  y: number;
  width: number;
  height: number;
  captureId: string | null;
  color: string | null;
}

export interface BoardSummary extends Board {
  objectCount: number;
  preview: PreviewItem[];
}

export type ObjectKind = "image" | "note";

export const NOTE_COLORS = ["sand", "sage", "sky", "rose", "lilac", "graphite"] as const;
export type NoteColor = (typeof NOTE_COLORS)[number];

export interface ObjectProps {
  text?: string;
  color?: NoteColor;
}

export interface BoardObject {
  id: string;
  boardId: string;
  kind: ObjectKind;
  captureId: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  z: number;
  groupId: string | null;
  props: ObjectProps;
  createdAt: number;
  updatedAt: number;
}

export interface Group {
  id: string;
  boardId: string;
  createdAt: number;
}

export interface BoardDocument {
  board: Board;
  objects: BoardObject[];
  groups: Group[];
}

export interface BoardChanges {
  boardId: string;
  upsertObjects: BoardObject[];
  deleteObjects: string[];
  upsertGroups: Group[];
  deleteGroups: string[];
}

export interface SaveReport {
  skippedObjects: string[];
}

export interface Settings {
  launchAtStartup: boolean;
  startMinimized: boolean;
  monitoring: boolean;
  shortcut: string;
  edgeEnabled: boolean;
  edgeDelayMs: number;
  panelAnimations: boolean;
  closeBehavior: "auto" | "manual";
  historyLimit: number;
  notifications: boolean;
  density: "comfortable" | "compact";
  effects: "off" | "subtle" | "full";
  reduceMotion: boolean;
  grid: "dots" | "none";
}

export interface StorageInfo {
  dataDir: string;
  captureCount: number;
  protectedCount: number;
  bytes: number;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  defaultDataDir: string;
  platform: string;
}

export type PasteResult =
  | { kind: "Image"; capture: Capture }
  | { kind: "Text"; text: string }
  | { kind: "Empty" };

export interface CapturesChanged {
  added?: Capture;
  updated?: Capture;
  removed: string[];
  reset: boolean;
}

export interface ObjectsInserted {
  boardId: string;
  objects: BoardObject[];
}

export interface ToastPayload {
  kind: "capture" | "info" | "error";
  capture?: Capture;
  message?: string;
}
