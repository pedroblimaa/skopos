import { useEffect, useRef, useState } from "react";
import { Dialog } from "../Dialog/Dialog";
import { Button } from "../Button/Button";
import { InfoTooltip } from "../InfoTooltip/InfoTooltip";
import { telegram } from "../../telegram";
import { appError, type AppMessage } from "../../app-message";
import type { useNotificationPreferences } from "./useNotificationPreferences";
import { useLanguage } from "../../i18n/useLanguage";
import "./NotificationSettings.css";

interface Props {
  onClose: () => void;
  cache: ReturnType<typeof useNotificationPreferences>;
}

export function NotificationSettings({ onClose, cache }: Props) {
  const { t, message, language } = useLanguage();
  const { settings, status } = cache;
  const [error, setError] = useState<AppMessage | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [isConfirmingRetry, setIsConfirmingRetry] = useState(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  const active = useRef(true);

  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);

  async function perform(operation: () => Promise<void>) {
    setIsBusy(true);
    setError(null);

    try {
      await operation();
    } catch (reason) {
      if (active.current) setError(appError(reason));
    } finally {
      if (active.current) setIsBusy(false);
    }
  }

  async function saveSwitch(key: "telegramEnabled" | "desktopEnabled", checked: boolean) {
    if (!settings) return;

    const next = await telegram.saveNotificationSettings({ ...settings, [key]: checked, language });

    if (active.current) cache.setSettings(next);
  }

  async function retry() {
    await telegram.retryUncertainNotifications();
    const next = await telegram.notificationStatus();

    if (!active.current) return;

    cache.updateStatus(next);
    setIsConfirmingRetry(false);
  }

  const failure = error ?? cache.error;

  return (
    <Dialog
      title={t("notifications")}
      headerAction={
        <InfoTooltip label={t("notificationsInfo")}>{t("notificationsHelp")}</InfoTooltip>
      }
      initialFocus={closeButton}
      isBusy={isBusy}
      onClose={onClose}
    >
      {(close) => (
        <>
          {settings && (
            <div className="notification-settings">
              <label className="notification-switch">
                <span>{t("telegramNotifications")}</span>
                <input
                  type="checkbox"
                  role="switch"
                  checked={settings.telegramEnabled}
                  disabled={isBusy}
                  onChange={(event) =>
                    void perform(() => saveSwitch("telegramEnabled", event.target.checked))
                  }
                />
              </label>
              <label className="notification-switch">
                <span>{t("desktopNotifications")}</span>
                <input
                  type="checkbox"
                  role="switch"
                  checked={settings.desktopEnabled}
                  disabled={isBusy}
                  onChange={(event) =>
                    void perform(() => saveSwitch("desktopEnabled", event.target.checked))
                  }
                />
              </label>
              {status && status.uncertain > 0 && (
                <div>
                  <p>{t("uncertainNotifications", { count: status.uncertain })}</p>
                  {isConfirmingRetry ? (
                    <>
                      <p>{t("retryNotificationsWarning")}</p>
                      <Button
                        type="button"
                        variant="quiet"
                        disabled={isBusy}
                        onClick={() => void perform(retry)}
                      >
                        {t("confirmRetryNotifications")}
                      </Button>
                    </>
                  ) : (
                    <Button
                      type="button"
                      variant="quiet"
                      disabled={isBusy}
                      onClick={() => {
                        setIsConfirmingRetry(true);
                      }}
                    >
                      {t("retryNotifications")}
                    </Button>
                  )}
                </div>
              )}
            </div>
          )}
          {failure && (
            <p className="error" role="alert">
              {message(failure)}
            </p>
          )}
          {!failure && status?.failure && (
            <p className="error" role="alert">
              {message(status.failure)}
            </p>
          )}
          {!settings && !failure && <p role="status">{t("loadingNotifications")}</p>}
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
