import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Copy, Minus, Square, X } from "lucide-react";
import { useLanguage } from "../../i18n/useLanguage";

type WindowAction = "minimize" | "toggleMaximize" | "close";

export function WindowControls() {
  const { t } = useLanguage();
  const [isMaximized, setIsMaximized] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [hasError, setHasError] = useState(false);
  const isDesktop = isTauri();

  useEffect(() => {
    if (!isDesktop) return;

    let isActive = true;
    let unlisten: UnlistenFn | undefined;
    const appWindow = getCurrentWindow();

    async function updateMaximized() {
      try {
        const maximized = await appWindow.isMaximized();
        if (isActive) setIsMaximized(maximized);
      } catch {
        if (isActive) setHasError(true);
      }
    }

    async function observeWindow() {
      try {
        const stop = await appWindow.onResized(() => void updateMaximized());
        if (!isActive) {
          stop();
          return;
        }

        unlisten = stop;
        await updateMaximized();
      } catch {
        if (isActive) setHasError(true);
      }
    }

    void observeWindow();

    return () => {
      isActive = false;
      unlisten?.();
    };
  }, [isDesktop]);

  async function runAction(action: WindowAction) {
    setIsBusy(true);
    setHasError(false);

    try {
      const appWindow = getCurrentWindow();
      await appWindow[action]();
      if (action === "toggleMaximize") setIsMaximized(await appWindow.isMaximized());
    } catch {
      setHasError(true);
    } finally {
      setIsBusy(false);
    }
  }

  if (!isDesktop) return null;

  return (
    <div className="window-controls" role="group" aria-label={t("windowControls")}>
      <button
        type="button"
        aria-label={t("minimizeWindow")}
        title={t("minimizeWindow")}
        disabled={isBusy}
        onClick={() => void runAction("minimize")}
      >
        <Minus size={16} aria-hidden="true" />
      </button>
      <button
        type="button"
        aria-label={t(isMaximized ? "restoreWindow" : "maximizeWindow")}
        title={t(isMaximized ? "restoreWindow" : "maximizeWindow")}
        disabled={isBusy}
        onClick={() => void runAction("toggleMaximize")}
      >
        {isMaximized ? (
          <Copy size={14} aria-hidden="true" />
        ) : (
          <Square size={14} aria-hidden="true" />
        )}
      </button>
      <button
        className="window-close"
        type="button"
        aria-label={t("closeWindow")}
        title={t("closeWindow")}
        disabled={isBusy}
        onClick={() => void runAction("close")}
      >
        <X size={18} aria-hidden="true" />
      </button>
      {hasError && (
        <p className="window-control-error" role="alert">
          {t("windowActionFailed")}
        </p>
      )}
    </div>
  );
}
