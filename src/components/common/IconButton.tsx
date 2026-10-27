import type { ButtonHTMLAttributes, ReactNode } from "react";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  children: ReactNode;
  active?: boolean;
  size?: "s" | "m";
  tone?: "default" | "danger";
}

/** Botón de ícono con tooltip nativo y estados hover/focus consistentes. */
export function IconButton({ label, children, active, size = "m", tone = "default", className = "", ...rest }: Props) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={`icon-btn icon-btn-${size} ${active ? "is-active" : ""} ${tone === "danger" ? "is-danger" : ""} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
