import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, X } from "lucide-react";
import { useCallback, useEffect, useState, type ReactNode } from "react";

import { flushSave } from "@/features/canvas/persistence";
import { api, errorMessage } from "@/services/api";
import { flash } from "@/stores/flashStore";
import { useSettings } from "@/stores/settingsStore";
import { useUi } from "@/stores/uiStore";
import type { AppInfo, Settings, StorageInfo } from "@/types/models";
import { formatBytes } from "@/utils/format";

import { IconButton } from "../common/IconButton";
import { Segmented } from "../common/Segmented";
import { Toggle } from "../common/Toggle";
import { ShortcutRecorder } from "./ShortcutRecorder";

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="setting">
      <div className="setting-text">
        <span className="setting-label">{label}</span>
        {hint && <span className="setting-hint">{hint}</span>}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

/** Ajustes: una sola hoja, cuatro secciones, solo lo necesario. */
export function SettingsSheet() {
  const s = useSettings((st) => st.settings);
  const update = useSettings((st) => st.update);
  const close = () => useUi.getState().setSettingsOpen(false);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [storage, setStorage] = useState<StorageInfo | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const [limitDraft, setLimitDraft] = useState<string>("");

  const refreshStorage = useCallback(() => void api.storageInfo().then(setStorage).catch(() => {}), []);
  useEffect(() => {
    void api.appInfo().then(setInfo).catch(() => {});
    refreshStorage();
  }, [refreshStorage]);
  useEffect(() => {
    if (s) setLimitDraft(String(s.historyLimit));
  }, [s?.historyLimit]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented) close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!s) return null;

  const set = async (patch: Partial<Settings>) => {
    const err = await update(patch);
    if (err) flash(err, "error");
    return err;
  };

  const commitLimit = () => {
    const n = Number.parseInt(limitDraft, 10);
    if (Number.isFinite(n) && n !== s.historyLimit) void set({ historyLimit: n }).then(refreshStorage);
    else setLimitDraft(String(s.historyLimit));
  };

  const clear = async () => {
    setConfirmClear(false);
    try {
      const n = await api.clearHistory();
      flash(n === 1 ? "Se eliminó 1 captura" : `Se eliminaron ${n} capturas`);
      refreshStorage();
    } catch (e) {
      flash(errorMessage(e), "error");
    }
  };

  const moveData = async () => {
    const dir = await openDialog({ directory: true, title: "Elegí la nueva carpeta para CanvasClip" });
    if (typeof dir !== "string") return;
    try {
      await flushSave();
      flash("Copiando datos… CanvasClip se reiniciará.");
      await api.changeDataDir(dir);
    } catch (e) {
      flash(errorMessage(e), "error");
    }
  };

  return (
    <div className="sheet-backdrop" onPointerDown={(e) => e.target === e.currentTarget && close()}>
      <section className="sheet" role="dialog" aria-modal="true" aria-label="Ajustes">
        <header className="sheet-header">
          <h2>Ajustes</h2>
          <IconButton label="Cerrar (Esc)" onClick={close}>
            <X size={16} strokeWidth={1.7} />
          </IconButton>
        </header>

        <div className="sheet-body">
          <h3>General</h3>
          <Row label="Iniciar con Windows">
            <Toggle label="Iniciar con Windows" checked={s.launchAtStartup} onChange={(v) => void set({ launchAtStartup: v })} />
          </Row>
          <Row label="Iniciar minimizado" hint="Al arrancar con Windows queda solo en la bandeja.">
            <Toggle label="Iniciar minimizado" checked={s.startMinimized} onChange={(v) => void set({ startMinimized: v })} />
          </Row>
          <Row label="Captura automática" hint="Guarda las imágenes que llegan al portapapeles.">
            <Toggle label="Captura automática" checked={s.monitoring} onChange={(v) => void set({ monitoring: v })} />
          </Row>

          <h3>Panel</h3>
          <Row label="Atajo global" hint="Abre y cierra el historial desde cualquier aplicación.">
            <ShortcutRecorder value={s.shortcut} onChange={(accel) => set({ shortcut: accel })} />
          </Row>
          <Row label="Abrir desde el borde derecho" hint="Apoyá el cursor en el borde de la pantalla.">
            <Toggle label="Abrir desde el borde" checked={s.edgeEnabled} onChange={(v) => void set({ edgeEnabled: v })} />
          </Row>
          <Row label="Tiempo de activación" hint={`${s.edgeDelayMs} ms en el borde antes de abrir.`}>
            <input
              type="range"
              className="range"
              min={100}
              max={1200}
              step={50}
              disabled={!s.edgeEnabled}
              defaultValue={s.edgeDelayMs}
              onPointerUp={(e) => void set({ edgeDelayMs: Number((e.target as HTMLInputElement).value) })}
              onKeyUp={(e) => void set({ edgeDelayMs: Number((e.target as HTMLInputElement).value) })}
              aria-label="Tiempo de activación del borde"
            />
          </Row>
          <Row label="Cierre" hint={s.closeBehavior === "auto" ? "Se cierra al hacer clic afuera o alejar el cursor." : "Solo con Esc, el atajo o el botón."}>
            <Segmented
              label="Comportamiento de cierre"
              value={s.closeBehavior}
              options={[
                { value: "auto", label: "Automático" },
                { value: "manual", label: "Manual" },
              ]}
              onChange={(v) => void set({ closeBehavior: v })}
            />
          </Row>
          <Row label="Animaciones del panel">
            <Toggle label="Animaciones del panel" checked={s.panelAnimations} onChange={(v) => void set({ panelAnimations: v })} />
          </Row>

          <h3>Capturas</h3>
          <Row label="Límite del historial" hint="Las favoritas y las usadas en pizarrones no cuentan y nunca se borran solas.">
            <input
              className="number"
              type="number"
              min={20}
              max={10000}
              step={50}
              value={limitDraft}
              onChange={(e) => setLimitDraft(e.target.value)}
              onBlur={commitLimit}
              onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
              aria-label="Límite del historial"
            />
          </Row>
          <Row label="Aviso al guardar una captura">
            <Toggle label="Aviso al guardar" checked={s.notifications} onChange={(v) => void set({ notifications: v })} />
          </Row>
          <Row
            label="Almacenamiento"
            hint={storage ? `${storage.captureCount} capturas (${storage.protectedCount} protegidas) · ${formatBytes(storage.bytes)}` : undefined}
          >
            <div className="setting-buttons">
              <IconButton label="Abrir carpeta" onClick={() => void api.openDataDir()}>
                <FolderOpen size={15} strokeWidth={1.7} />
              </IconButton>
              <button type="button" className="btn" onClick={moveData}>
                Cambiar…
              </button>
            </div>
          </Row>
          {info && <p className="setting-path" title={info.dataDir}>{info.dataDir}</p>}
          <Row label="Limpiar historial" hint="Borra las capturas no protegidas. No se puede deshacer.">
            {confirmClear ? (
              <div className="setting-buttons">
                <button type="button" className="btn btn-ghost" onClick={() => setConfirmClear(false)}>Cancelar</button>
                <button type="button" className="btn btn-danger" onClick={clear}>Borrar</button>
              </div>
            ) : (
              <button type="button" className="btn" onClick={() => setConfirmClear(true)}>Limpiar…</button>
            )}
          </Row>

          <h3>Apariencia</h3>
          <Row label="Tema" hint="Oscuro (predeterminado).">
            <span className="setting-static">Oscuro</span>
          </Row>
          <Row label="Densidad">
            <Segmented
              label="Densidad"
              value={s.density}
              options={[
                { value: "comfortable", label: "Cómoda" },
                { value: "compact", label: "Compacta" },
              ]}
              onChange={(v) => void set({ density: v })}
            />
          </Row>
          <Row label="Transparencias" hint="Vidrio esmerilado del panel y los avisos.">
            <Segmented
              label="Intensidad de efectos"
              value={s.effects}
              options={[
                { value: "off", label: "No" },
                { value: "subtle", label: "Sutil" },
                { value: "full", label: "Plena" },
              ]}
              onChange={(v) => void set({ effects: v })}
            />
          </Row>
          <Row label="Cuadrícula del pizarrón">
            <Segmented
              label="Cuadrícula"
              value={s.grid}
              options={[
                { value: "dots", label: "Puntos" },
                { value: "none", label: "Lisa" },
              ]}
              onChange={(v) => void set({ grid: v })}
            />
          </Row>
          <Row label="Reducir animaciones">
            <Toggle label="Reducir animaciones" checked={s.reduceMotion} onChange={(v) => void set({ reduceMotion: v })} />
          </Row>

          {info && <p className="sheet-footer">CanvasClip {info.version}</p>}
        </div>
      </section>
    </div>
  );
}
