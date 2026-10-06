import { useEffect, useRef, useState } from "react";
import { Dialog } from "../Dialog/Dialog";
import { Button } from "../Button/Button";
import { telegram } from "../../telegram";
import { appError, type AppMessage } from "../../app-message";
import type {
  NotificationSettings as Settings,
  NotificationStatus,
} from "../../notification.model";
import { useLanguage } from "../../i18n/useLanguage";
import "./NotificationSettings.css";

export function NotificationSettings({ onClose }: { onClose: () => void }) {
  const { t, message, language } = useLanguage();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<NotificationStatus | null>(null);
  const [error, setError] = useState<AppMessage | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [isConfirmingRetry, setIsConfirmingRetry] = useState(false);
  const active = useRef(true);

  useEffect(() => {
    active.current = true;
    async function load() {
      try {
        const [next, delivery] = await Promise.all([
          telegram.notificationSettings(),
          telegram.notificationStatus(),
        ]);
        if (!active.current) return;

        setSettings(next);
        setStatus(delivery);
      } catch (reason) {
        if (active.current) setError(appError(reason));
      }
    }
    void load();
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
    if (active.current) setSettings(next);
  }

  async function retry() {
    await telegram.retryUncertainNotifications();
    const next = await telegram.notificationStatus();
    if (!active.current) return;

    setStatus(next);
    setIsConfirmingRetry(false);
  }

  return (
    <Dialog title={t("notifications")} isBusy={isBusy} onClose={onClose}>
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
          <p>{t("notificationsHelp")}</p>
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
      {error && (
        <p className="error" role="alert">
          {message(error)}
        </p>
      )}
      {!error && status?.failure && (
        <p className="error" role="alert">
          {message(status.failure)}
        </p>
      )}
      {!settings && !error && <p role="status">{t("loadingNotifications")}</p>}
      <div className="dialog-actions">
        <Button type="button" variant="quiet" disabled={isBusy} onClick={onClose}>
          {t("closeNotifications")}
        </Button>
      </div>
    </Dialog>
  );
}
