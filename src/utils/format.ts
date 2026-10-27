const rtf = new Intl.RelativeTimeFormat("es", { numeric: "auto" });
const timeFmt = new Intl.DateTimeFormat("es", { hour: "2-digit", minute: "2-digit" });
const dateFmt = new Intl.DateTimeFormat("es", { day: "numeric", month: "short" });

/** "hace 3 min", "ayer, 14:20", "12 mar". */
export function relativeTime(ms: number, now = Date.now()): string {
  const diff = Math.round((ms - now) / 1000);
  const abs = Math.abs(diff);
  if (abs < 45) return "ahora";
  if (abs < 3600) return rtf.format(Math.round(diff / 60), "minute");
  const d = new Date(ms);
  const today = new Date(now);
  const sameDay = d.toDateString() === today.toDateString();
  if (sameDay) return rtf.format(Math.round(diff / 3600), "hour");
  const yesterday = new Date(now - 86_400_000);
  if (d.toDateString() === yesterday.toDateString()) return `ayer, ${timeFmt.format(d)}`;
  return dateFmt.format(d);
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

export function formatDims(w: number, h: number): string {
  return `${w}×${h}`;
}
