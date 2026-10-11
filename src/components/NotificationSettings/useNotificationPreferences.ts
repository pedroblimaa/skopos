import { useEffect, useRef, useState } from "react";
import { appError, type AppMessage } from "../../app-message";
import { telegram } from "../../telegram";
import type { NotificationSettings, NotificationStatus } from "../../notification.model";

export function useNotificationPreferences(isEnabled: boolean) {
  const [settings, setSettings] = useState<NotificationSettings | null>(null);
  const [status, setStatus] = useState<NotificationStatus | null>(null);
  const [settingsError, setSettingsError] = useState<AppMessage | null>(null);
  const [statusError, setStatusError] = useState<AppMessage | null>(null);
  const [loadRevision, setLoadRevision] = useState(0);
  const generation = useRef(0);
  const statusRevision = useRef(0);

  useEffect(() => {
    const current = ++generation.current;
    setSettings(null);
    setStatus(null);
    setSettingsError(null);
    setStatusError(null);

    if (!isEnabled) return;

    const revision = statusRevision.current;
    const subscription = telegram.onNotificationStatus((next) => {
      if (generation.current !== current) return;

      statusRevision.current += 1;
      setStatus(next);
      setStatusError(null);
    });
    void subscription.catch((reason: unknown) => {
      if (generation.current === current) setStatusError(appError(reason));
    });

    async function loadSettings() {
      try {
        const next = await telegram.notificationSettings();

        if (generation.current === current) setSettings(next);
      } catch (reason) {
        if (generation.current === current) setSettingsError(appError(reason));
      }
    }

    async function loadStatus() {
      try {
        const next = await telegram.notificationStatus();

        if (generation.current === current && statusRevision.current === revision) setStatus(next);
      } catch (reason) {
        if (generation.current === current && statusRevision.current === revision) {
          setStatusError(appError(reason));
        }
      }
    }

    void loadSettings();
    void loadStatus();
    return () => {
      generation.current += 1;
      void subscription.then(
        (unsubscribe) => {
          unsubscribe();
        },
        () => {},
      );
    };
  }, [isEnabled, loadRevision]);

  function reload() {
    setLoadRevision((previous) => previous + 1);
  }

  function updateStatus(next: NotificationStatus) {
    statusRevision.current += 1;
    setStatus(next);
    setStatusError(null);
  }

  return {
    settings,
    status,
    error: settingsError ?? statusError,
    setSettings,
    updateStatus,
    reload,
  };
}
