import { useEffect, useRef, useState } from "react";
import { Dialog } from "../Dialog/Dialog";
import { Button } from "../Button/Button";
import { InfoTooltip } from "../InfoTooltip/InfoTooltip";
import { Switch } from "../Switch/Switch";
import { telegram } from "../../telegram";
import { appError, type AppMessage } from "../../app-message";
import type { useMonitoringPreferences } from "./useMonitoringPreferences";
import { useLanguage } from "../../i18n/useLanguage";
import "./MonitoringSettings.css";

interface Props {
  onClose: () => void;
  cache: ReturnType<typeof useMonitoringPreferences>;
}

export function MonitoringSettings({ onClose, cache }: Props) {
  const { t, message, language } = useLanguage();
  const { status, startup } = cache;
  const [error, setError] = useState<AppMessage | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  const active = useRef(true);

  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);

  async function save(kind: "monitoring" | "startup", enabled: boolean) {
    setIsBusy(true);
    setError(null);

    try {
      if (kind === "monitoring") {
        const next = await telegram.saveMonitoringSettings(enabled);

        if (active.current) cache.updateStatus(next);
      } else {
        const next = await telegram.saveStartupSettings(enabled);

        if (active.current) cache.setStartup(next);
      }
    } catch (reason) {
      if (active.current) setError(appError(reason));
    } finally {
      if (active.current) setIsBusy(false);
    }
  }

  function renderTimestamp(timestamp: number | null) {
    if (timestamp === null) return t("monitoringNever");

    const value = new Date(timestamp * 1000);

    return (
      <time dateTime={value.toISOString()}>
        <span className="monitoring-time">
          {value.toLocaleTimeString(language, {
            hour: "2-digit",
            minute: "2-digit",
            hour12: false,
          })}
        </span>
        <span className="monitoring-date">
          {value.toLocaleDateString(language, { dateStyle: "medium" })}
        </span>
      </time>
    );
  }

  const failure = error ?? cache.error;

  return (
    <Dialog
      title={t("monitoring")}
      headerAction={<InfoTooltip label={t("monitoringInfo")}>{t("monitoringDetails")}</InfoTooltip>}
      initialFocus={closeButton}
      isBusy={isBusy}
      onClose={onClose}
    >
      {(close) => (
        <>
          {(status || startup) && (
            <div className="monitoring-settings">
              {status && (
                <Switch
                  label={t("automaticMonitoring")}
                  checked={status.enabled}
                  disabled={isBusy}
                  onChange={(checked) => void save("monitoring", checked)}
                />
              )}
              {startup && (
                <Switch
                  label={t("startWithWindows")}
                  checked={startup.enabled}
                  disabled={isBusy}
                  onChange={(checked) => void save("startup", checked)}
                />
              )}
              {status && (
                <dl className="monitoring-times">
                  <div>
                    <dt>{t("monitoringLastAttempt")}</dt>
                    <dd>{renderTimestamp(status.lastAttempt)}</dd>
                  </div>
                  <div>
                    <dt>{t("monitoringNextDue")}</dt>
                    <dd>
                      {status.enabled ? renderTimestamp(status.nextDue) : t("monitoringPaused")}
                    </dd>
                  </div>
                </dl>
              )}
              {status?.isRunning && <p role="status">{t("monitoringRunning")}</p>}
              {status?.failure && (
                <p className="error" role="alert">
                  {message(status.failure)}
                </p>
              )}
              {startup?.failure && (
                <p className="error" role="alert">
                  {message(startup.failure)}
                </p>
              )}
            </div>
          )}
          {failure && (
            <p className="error" role="alert">
              {message(failure)}
            </p>
          )}
          {(!status || !startup) && !failure && <p role="status">{t("loadingMonitoring")}</p>}
          <div className="dialog-actions">
            <Button
              type="button"
              variant="quiet"
              ref={closeButton}
              disabled={isBusy}
              onClick={close}
            >
              {t("closeNotifications")}
            </Button>
          </div>
        </>
      )}
    </Dialog>
  );
}
