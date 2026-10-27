import { ChevronLeft, ChevronRight } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

export interface MenuItem {
  id: string;
  label: string;
  icon?: ReactNode;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  /** Abre un submenú dentro del mismo menú (con botón para volver). */
  submenu?: () => MenuItem[];
  onSelect?: () => void;
  /** Separador antes de este ítem. */
  separator?: boolean;
  /** Contenido propio en lugar de un ítem estándar (p. ej., muestras de color). */
  render?: (close: () => void) => ReactNode;
}

interface Props {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

/** Menú contextual compacto. Se ajusta para no salir de la ventana. */
export function Menu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [stack, setStack] = useState<{ title: string; items: MenuItem[] }[]>([]);
  const [pos, setPos] = useState({ x, y });
  const [active, setActive] = useState(-1);
  const current = stack.length ? stack[stack.length - 1].items : items;

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setPos({
      x: Math.max(6, Math.min(x, window.innerWidth - r.width - 6)),
      y: Math.max(6, Math.min(y, window.innerHeight - r.height - 6)),
    });
  }, [x, y, stack.length]);

  useEffect(() => {
    const onDown = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      const enabled = current.map((it, i) => (it.disabled || it.render ? -1 : i)).filter((i) => i >= 0);
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (stack.length) setStack((s) => s.slice(0, -1));
        else onClose();
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const idx = enabled.indexOf(active);
        const next = e.key === "ArrowDown" ? enabled[(idx + 1) % enabled.length] : enabled[(idx - 1 + enabled.length) % enabled.length];
        setActive(next ?? -1);
      } else if (e.key === "Enter" && active >= 0) {
        e.preventDefault();
        choose(current[active]);
      } else if (e.key === "ArrowLeft" && stack.length) {
        setStack((s) => s.slice(0, -1));
      }
    };
    const onBlur = () => onClose();
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", onBlur);
    };
  });

  function choose(item: MenuItem) {
    if (item.disabled) return;
    if (item.submenu) {
      setStack((s) => [...s, { title: item.label, items: item.submenu!() }]);
      setActive(-1);
      return;
    }
    onClose();
    item.onSelect?.();
  }

  return createPortal(
    <div ref={ref} className="menu" role="menu" style={{ left: pos.x, top: pos.y }} onContextMenu={(e) => e.preventDefault()}>
      {stack.length > 0 && (
        <button type="button" className="menu-back" onClick={() => setStack((s) => s.slice(0, -1))}>
          <ChevronLeft size={14} strokeWidth={1.8} />
          <span>{stack[stack.length - 1].title}</span>
        </button>
      )}
      {current.map((item, i) => (
        <div key={item.id}>
          {item.separator && <div className="menu-sep" role="separator" />}
          {item.render ? (
            item.render(onClose)
          ) : (
            <button
              type="button"
              role="menuitem"
              disabled={item.disabled}
              className={`menu-item ${item.danger ? "is-danger" : ""} ${i === active ? "is-active" : ""}`}
              onPointerEnter={() => setActive(i)}
              onClick={() => choose(item)}
            >
              <span className="menu-icon">{item.icon}</span>
              <span className="menu-label">{item.label}</span>
              {item.shortcut && <span className="menu-shortcut">{item.shortcut}</span>}
              {item.submenu && <ChevronRight size={14} strokeWidth={1.8} className="menu-chevron" />}
            </button>
          )}
        </div>
      ))}
      {current.length === 0 && <div className="menu-empty">Sin opciones</div>}
    </div>,
    document.body,
  );
}
