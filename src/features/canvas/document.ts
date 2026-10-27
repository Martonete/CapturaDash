// Documento de un pizarrón: operaciones puras e inmutables sobre objetos y
// grupos. Cada operación devuelve un documento nuevo (útil para deshacer).

import type { BoardChanges, BoardDocument, BoardObject, Group, NoteColor } from "@/types/models";

import type { Rect } from "./geometry";

export interface Doc {
  objects: Record<string, BoardObject>;
  groups: Record<string, Group>;
}

export const emptyDoc = (): Doc => ({ objects: {}, groups: {} });

export function fromBoardDocument(d: BoardDocument): Doc {
  const objects: Record<string, BoardObject> = {};
  for (const o of d.objects) objects[o.id] = { ...o, props: o.props ?? {} };
  const groups: Record<string, Group> = {};
  for (const g of d.groups) groups[g.id] = g;
  return normalizeGroups({ objects, groups });
}

export function sortedObjects(doc: Doc): BoardObject[] {
  return Object.values(doc.objects).sort((a, b) => a.z - b.z || (a.id < b.id ? -1 : 1));
}

export function maxZ(doc: Doc): number {
  let z = 0;
  for (const o of Object.values(doc.objects)) z = Math.max(z, o.z);
  return z;
}

export const rectOf = (o: BoardObject): Rect => ({ x: o.x, y: o.y, width: o.width, height: o.height });

export function membersOf(doc: Doc, groupId: string): string[] {
  return Object.values(doc.objects)
    .filter((o) => o.groupId === groupId)
    .map((o) => o.id);
}

/**
 * Expande una selección a grupos completos. Si `isolatedGroup` coincide con el
 * grupo de un objeto, ese objeto se selecciona solo (edición individual).
 */
export function expandToGroups(doc: Doc, ids: Iterable<string>, isolatedGroup: string | null = null): string[] {
  const out = new Set<string>();
  for (const id of ids) {
    const o = doc.objects[id];
    if (!o) continue;
    if (o.groupId && o.groupId !== isolatedGroup) {
      for (const m of membersOf(doc, o.groupId)) out.add(m);
    } else {
      out.add(id);
    }
  }
  return [...out];
}

function touch(o: BoardObject, patch: Partial<BoardObject>): BoardObject {
  return { ...o, ...patch, updatedAt: Date.now() };
}

export function addObjects(doc: Doc, objs: BoardObject[]): Doc {
  const objects = { ...doc.objects };
  for (const o of objs) objects[o.id] = o;
  return { ...doc, objects };
}

export function updateObjects(doc: Doc, patches: Record<string, Partial<BoardObject>>): Doc {
  const objects = { ...doc.objects };
  let changed = false;
  for (const [id, patch] of Object.entries(patches)) {
    const o = objects[id];
    if (!o) continue;
    objects[id] = touch(o, patch);
    changed = true;
  }
  return changed ? { ...doc, objects } : doc;
}

export function removeObjects(doc: Doc, ids: Iterable<string>): Doc {
  const objects = { ...doc.objects };
  let changed = false;
  for (const id of ids) {
    if (objects[id]) {
      delete objects[id];
      changed = true;
    }
  }
  return changed ? normalizeGroups({ ...doc, objects }) : doc;
}

/** Elimina grupos con menos de dos miembros y referencias a grupos inexistentes. */
export function normalizeGroups(doc: Doc): Doc {
  const counts = new Map<string, number>();
  for (const o of Object.values(doc.objects)) {
    if (o.groupId) counts.set(o.groupId, (counts.get(o.groupId) ?? 0) + 1);
  }
  let objects = doc.objects;
  let groups = doc.groups;
  const dead = new Set<string>();
  for (const id of Object.keys(groups)) if ((counts.get(id) ?? 0) < 2) dead.add(id);
  for (const id of counts.keys()) if (!groups[id] || (counts.get(id) ?? 0) < 2) dead.add(id);
  if (dead.size === 0) return doc;

  groups = { ...groups };
  for (const id of dead) delete groups[id];
  objects = { ...objects };
  for (const o of Object.values(objects)) {
    if (o.groupId && dead.has(o.groupId)) objects[o.id] = touch(o, { groupId: null });
  }
  return { objects, groups };
}

/** Agrupa los objetos indicados (expandidos a sus grupos actuales) en un grupo nuevo. */
export function groupObjects(doc: Doc, ids: string[], group: Group): Doc {
  const all = expandToGroups(doc, ids);
  if (all.length < 2) return doc;
  const objects = { ...doc.objects };
  for (const id of all) objects[id] = touch(objects[id], { groupId: group.id });
  return normalizeGroups({ objects, groups: { ...doc.groups, [group.id]: group } });
}

export function ungroupObjects(doc: Doc, ids: string[]): Doc {
  const groupIds = new Set(ids.map((id) => doc.objects[id]?.groupId).filter((g): g is string => !!g));
  if (groupIds.size === 0) return doc;
  const objects = { ...doc.objects };
  for (const o of Object.values(objects)) {
    if (o.groupId && groupIds.has(o.groupId)) objects[o.id] = touch(o, { groupId: null });
  }
  const groups = { ...doc.groups };
  for (const g of groupIds) delete groups[g];
  return { objects, groups };
}

/** Lleva al frente (o al fondo) conservando el orden relativo de la selección. */
export function reorder(doc: Doc, ids: string[], where: "front" | "back"): Doc {
  const set = new Set(ids);
  const ordered = sortedObjects(doc);
  const moving = ordered.filter((o) => set.has(o.id));
  const rest = ordered.filter((o) => !set.has(o.id));
  const next = where === "front" ? [...rest, ...moving] : [...moving, ...rest];
  const patches: Record<string, Partial<BoardObject>> = {};
  next.forEach((o, i) => {
    if (o.z !== i) patches[o.id] = { z: i };
  });
  return updateObjects(doc, patches);
}

export function setNoteColor(doc: Doc, ids: string[], color: NoteColor): Doc {
  const patches: Record<string, Partial<BoardObject>> = {};
  for (const id of ids) {
    const o = doc.objects[id];
    if (o?.kind === "note") patches[id] = { props: { ...o.props, color } };
  }
  return updateObjects(doc, patches);
}

function sameObject(a: BoardObject, b: BoardObject): boolean {
  return (
    a.x === b.x &&
    a.y === b.y &&
    a.width === b.width &&
    a.height === b.height &&
    a.z === b.z &&
    a.groupId === b.groupId &&
    a.props.text === b.props.text &&
    a.props.color === b.props.color
  );
}

/** Cambios mínimos para pasar de `saved` (lo que hay en disco) a `current`. */
export function diffDocs(saved: Doc, current: Doc, boardId: string): BoardChanges {
  const changes: BoardChanges = { boardId, upsertObjects: [], deleteObjects: [], upsertGroups: [], deleteGroups: [] };
  for (const [id, o] of Object.entries(current.objects)) {
    const prev = saved.objects[id];
    if (!prev || (prev !== o && !sameObject(prev, o))) changes.upsertObjects.push(o);
  }
  for (const id of Object.keys(saved.objects)) if (!current.objects[id]) changes.deleteObjects.push(id);
  for (const [id, g] of Object.entries(current.groups)) if (!saved.groups[id]) changes.upsertGroups.push(g);
  for (const id of Object.keys(saved.groups)) if (!current.groups[id]) changes.deleteGroups.push(id);
  return changes;
}

export function isEmptyChange(c: BoardChanges): boolean {
  return !c.upsertObjects.length && !c.deleteObjects.length && !c.upsertGroups.length && !c.deleteGroups.length;
}
