import { Copy, LayoutGrid, Plus, Star, StarOff, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { addToBoard, copyCapture, deleteCapture, toggleFavorite } from "@/features/history/actions";
import { useBoards } from "@/stores/boardsStore";
import { useHistory } from "@/stores/historyStore";
import type { Capture } from "@/types/models";

import { Menu, type MenuItem } from "../common/Menu";
import { Segmented } from "../common/Segmented";
import { CaptureCard } from "./CaptureCard";

interface Props {
  /** Teclado activo (flechas, Enter, Supr) cuando la lista tiene el foco lógico. */
  keyboard: boolean;
  emptyHint: string;
}

/** Lista vertical del historial con filtro, carga progresiva y navegación por teclado. */
export function CaptureList({ keyboard, emptyHint }: Props) {
  const { items, favoritesOnly, loading, hasMore, error, loadMore, setFavoritesOnly } = useHistory();
  const boards = useBoards((s) => s.boards);
  const activeId = useBoards((s) => s.activeId);
  const [selected, setSelected] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ c: Capture; x: number; y: number } | null>(null);
  const [now, setNow] = useState(Date.now());
  const scrollRef = useRef<HTMLDivElement>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);

  // Las horas relativas se actualizan cada 30 s.
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(t);
  }, []);

  // Carga progresiva al acercarse al final.
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const io = new IntersectionObserver((e) => e[0]?.isIntersecting && void loadMore(), { root: scrollRef.current, rootMargin: "600px" });
    io.observe(el);
    return () => io.disconnect();
  }, [loadMore]);

  const current = items.find((c) => c.id === selected) ?? null;

  useEffect(() => {
    if (!keyboard) return;
    const onKey = (e: KeyboardEvent) => {
      if (menu || (e.target as HTMLElement)?.tagName === "INPUT") return;
      const idx = items.findIndex((c) => c.id === selected);
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const next = e.key === "ArrowDown" ? Math.min(items.length - 1, idx + 1) : Math.max(0, idx - 1);
        const c = items[next];
        if (c) {
          setSelected(c.id);
          scrollRef.current?.querySelector(`[data-capture-id="${c.id}"]`)?.scrollIntoView({ block: "nearest" });
        }
      } else if (e.key === "Enter" && current) {
        e.preventDefault();
        void copyCapture(current);
      } else if ((e.key === "Delete" || e.key === "Backspace") && current) {
        e.preventDefault();
        setConfirming(current.id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [keyboard, items, selected, current, menu]);

  const onDelete = useCallback((c: Capture) => {
    setConfirming(null);
    void deleteCapture(c, c.usageCount > 0);
  }, []);
  const onConfirm = useCallback((c: Capture | null) => setConfirming(c?.id ?? null), []);
  const onMenu = useCallback((c: Capture, x: number, y: number) => {
    setSelected(c.id);
    setMenu({ c, x, y });
  }, []);
  const onAdd = useCallback((c: Capture) => void addToBoard(c), []);

  const menuItems = (c: Capture): MenuItem[] => [
    { id: "copy", label: "Copiar", icon: <Copy size={14} />, shortcut: "Enter", onSelect: () => void copyCapture(c) },
    { id: "add", label: "Agregar al pizarrón actual", icon: <Plus size={14} />, onSelect: () => void addToBoard(c) },
    {
      id: "add-to",
      label: "Agregar a otro pizarrón",
      icon: <LayoutGrid size={14} />,
      submenu: () =>
        boards.map((b) => ({
          id: b.id,
          label: b.id === activeId ? `${b.name} (actual)` : b.name,
          onSelect: () => void addToBoard(c, b.id),
        })),
    },
    {
      id: "fav",
      label: c.favorite ? "Quitar de favoritas" : "Marcar como favorita",
      icon: c.favorite ? <StarOff size={14} /> : <Star size={14} />,
      onSelect: () => void toggleFavorite(c),
    },
    { id: "del", label: "Eliminar…", icon: <Trash2 size={14} />, danger: true, separator: true, shortcut: "Supr", onSelect: () => setConfirming(c.id) },
  ];

  return (
    <div className="capture-list-wrap">
      <div className="capture-filter">
        <Segmented
          size="s"
          label="Filtro del historial"
          value={favoritesOnly ? "fav" : "all"}
          options={[
            { value: "all", label: "Recientes" },
            { value: "fav", label: "Favoritas" },
          ]}
          onChange={(v) => setFavoritesOnly(v === "fav")}
        />
      </div>
      <div className="capture-list" ref={scrollRef}>
        {items.map((c) => (
          <CaptureCard
            key={c.id}
            capture={c}
            now={now}
            selected={c.id === selected}
            confirmingDelete={confirming === c.id}
            onFocusCard={setSelected}
            onCopy={copyCapture}
            onAdd={onAdd}
            onMenu={onMenu}
            onConfirmDelete={onConfirm}
            onDelete={onDelete}
          />
        ))}
        {!loading && items.length === 0 && !error && (
          <div className="capture-empty">
            <p>{favoritesOnly ? "Todavía no marcaste capturas como favoritas." : emptyHint}</p>
          </div>
        )}
        {error && <div className="capture-empty is-error">{error}</div>}
        <div ref={sentinelRef} className="capture-sentinel" aria-hidden>
          {hasMore && items.length > 0 ? "…" : null}
        </div>
      </div>
      {menu && <Menu x={menu.x} y={menu.y} items={menuItems(menu.c)} onClose={() => setMenu(null)} />}
    </div>
  );
}
