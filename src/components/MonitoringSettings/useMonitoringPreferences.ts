import { useEffect, useRef, useState } from "react";
import { appError, type AppMessage } from "../../app-message";
import { telegram } from "../../telegram";
import type { MonitoringStatus, StartupSettings } from "../../monitoring.model";

export function useMonitoringPreferences(isEnabled: boolean) {
  const [status, setStatus] = useState<MonitoringStatus | null>(null);
  const [startup, setStartup] = useState<StartupSettings | null>(null);
  const [monitoringError, setMonitoringError] = useState<AppMessage | null>(null);
  const [startupError, setStartupError] = useState<AppMessage | null>(null);
  const [loadRevision, setLoadRevision] = useState(0);
  const generation = useRef(0);
  const revision = useRef(0);

  useEffect(() => {
    const current = ++generation.current;
    setStatus(null);
    setStartup(null);
    setMonitoringError(null);
    setStartupError(null);

    if (!isEnabled) return;

    const startedRevision = revision.current;
    const subscription = telegram.onMonitoringStatus((next) => {
      if (generation.current !== current) return;

      revision.current += 1;
      setStatus((previous) =>
        previous && previous.accountId !== next.accountId ? previous : next,
      );
      setMonitoringError(null);
    });
    void subscription.catch((reason: unknown) => {
      if (generation.current === current) setMonitoringError(appError(reason));
    });

    async function loadStatus() {
      try {
        const next = await telegram.monitoringStatus();

        if (generation.current === current && revision.current === startedRevision) setStatus(next);
      } catch (reason) {
        if (generation.current === current && revision.current === startedRevision) {
          setMonitoringError(appError(reason));
        }
      }
    }

    async function loadStartup() {
      try {
        const next = await telegram.startupSettings();

        if (generation.current === current) setStartup(next);
      } catch (reason) {
        if (generation.current === current) setStartupError(appError(reason));
      }
    }

    void loadStatus();
    void loadStartup();
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

  function updateStatus(next: MonitoringStatus) {
    revision.current += 1;
    setStatus(next);
    setMonitoringError(null);
  }

  return {
    status,
    startup,
    error: monitoringError ?? startupError,
    updateStatus,
    setStartup,
    reload,
  };
}
