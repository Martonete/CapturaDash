import { describe, expect, it } from "vitest";

import {
  addObjects,
  diffDocs,
  emptyDoc,
  expandToGroups,
  groupObjects,
  isEmptyChange,
  normalizeGroups,
  removeObjects,
  reorder,
  setNoteColor,
  sortedObjects,
  ungroupObjects,
  updateObjects,
  type Doc,
} from "@/features/canvas/document";
import type { BoardObject } from "@/types/models";

const obj = (id: string, z = 0, kind: BoardObject["kind"] = "image"): BoardObject => ({
  id,
  boardId: "b",
  kind,
  captureId: kind === "image" ? `cap-${id}` : null,
  x: 0,
  y: 0,
  width: 10,
  height: 10,
  z,
  groupId: null,
  props: kind === "note" ? { text: "", color: "sand" } : {},
  createdAt: 0,
  updatedAt: 0,
});

const g = (id: string) => ({ id, boardId: "b", createdAt: 0 });

function docWith(...objs: BoardObject[]): Doc {
  return addObjects(emptyDoc(), objs);
}

describe("grupos", () => {
  it("agrupar, expandir selección y desagrupar", () => {
    let d = docWith(obj("a"), obj("b"), obj("c"));
    d = groupObjects(d, ["a", "b"], g("g1"));
    expect(d.objects.a.groupId).toBe("g1");
    expect(expandToGroups(d, ["a"]).sort()).toEqual(["a", "b"]);
    // Aislado: se edita el miembro por separado.
    expect(expandToGroups(d, ["a"], "g1")).toEqual(["a"]);
    d = ungroupObjects(d, ["b"]);
    expect(d.objects.a.groupId).toBeNull();
    expect(d.groups.g1).toBeUndefined();
  });

  it("agrupar grupos existentes los fusiona en uno nuevo", () => {
    let d = docWith(obj("a"), obj("b"), obj("c"), obj("d"));
    d = groupObjects(d, ["a", "b"], g("g1"));
    d = groupObjects(d, ["c", "a"], g("g2"));
    expect(Object.keys(d.groups)).toEqual(["g2"]);
    expect(["a", "b", "c"].every((id) => d.objects[id].groupId === "g2")).toBe(true);
    expect(d.objects.d.groupId).toBeNull();
  });

  it("un grupo que queda con un solo miembro se disuelve", () => {
    let d = docWith(obj("a"), obj("b"));
    d = groupObjects(d, ["a", "b"], g("g1"));
    d = removeObjects(d, ["a"]);
    expect(d.groups.g1).toBeUndefined();
    expect(d.objects.b.groupId).toBeNull();
  });

  it("normaliza referencias a grupos inexistentes", () => {
    const d = docWith({ ...obj("a"), groupId: "fantasma" }, { ...obj("b"), groupId: "fantasma" });
    const n = normalizeGroups(d);
    expect(n.objects.a.groupId).toBeNull();
  });
});

describe("orden y propiedades", () => {
  it("al frente / al fondo conserva el orden relativo", () => {
    let d = docWith(obj("a", 0), obj("b", 1), obj("c", 2));
    d = reorder(d, ["a"], "front");
    expect(sortedObjects(d).map((o) => o.id)).toEqual(["b", "c", "a"]);
    d = reorder(d, ["c", "a"], "back");
    expect(sortedObjects(d).map((o) => o.id)).toEqual(["c", "a", "b"]);
  });

  it("solo cambia el color de las notas", () => {
    let d = docWith(obj("n", 0, "note"), obj("i"));
    d = setNoteColor(d, ["n", "i"], "sky");
    expect(d.objects.n.props.color).toBe("sky");
    expect(d.objects.i.props.color).toBeUndefined();
  });

  it("las operaciones sin efecto devuelven el mismo documento", () => {
    const d = docWith(obj("a"));
    expect(updateObjects(d, { zz: { x: 1 } })).toBe(d);
    expect(removeObjects(d, ["zz"])).toBe(d);
    expect(ungroupObjects(d, ["a"])).toBe(d);
  });
});

describe("diff para guardar", () => {
  it("detecta altas, cambios, bajas y grupos", () => {
    const saved = docWith(obj("a"), obj("b"));
    let cur = updateObjects(saved, { a: { x: 50 } });
    cur = removeObjects(cur, ["b"]);
    cur = addObjects(cur, [obj("c")]);
    const ch = diffDocs(saved, cur, "b");
    expect(ch.upsertObjects.map((o) => o.id).sort()).toEqual(["a", "c"]);
    expect(ch.deleteObjects).toEqual(["b"]);

    const grouped = groupObjects(cur, ["a", "c"], g("g1"));
    const ch2 = diffDocs(cur, grouped, "b");
    expect(ch2.upsertGroups.map((x) => x.id)).toEqual(["g1"]);
    expect(ch2.upsertObjects).toHaveLength(2);
    const ch3 = diffDocs(grouped, ungroupObjects(grouped, ["a"]), "b");
    expect(ch3.deleteGroups).toEqual(["g1"]);
  });

  it("sin cambios reales no genera escrituras", () => {
    const d = docWith(obj("a"));
    const same = updateObjects(d, { a: { x: 0 } }); // mismo valor, objeto nuevo
    expect(isEmptyChange(diffDocs(d, same, "b"))).toBe(true);
  });
});
