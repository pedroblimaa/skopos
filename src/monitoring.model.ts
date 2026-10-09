import type { AppMessage } from "./app-message";

export interface MonitoringStatus {
  accountId: number;
  enabled: boolean;
  isRunning: boolean;
  lastAttempt: number | null;
  nextDue: number | null;
  failure: AppMessage | null;
}

export interface StartupSettings {
  enabled: boolean;
  failure: AppMessage | null;
}
