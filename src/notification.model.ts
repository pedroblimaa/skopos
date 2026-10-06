import type { AppMessage } from "./app-message";

export interface NotificationSettings {
  telegramEnabled: boolean;
  desktopEnabled: boolean;
  language: string;
}

export interface NotificationStatus {
  pending: number;
  uncertain: number;
  failure: AppMessage | null;
}
