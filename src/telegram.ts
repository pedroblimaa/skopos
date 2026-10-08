import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CodeRequest,
  CodeSubmissionError,
  LoginResult,
  QrToken,
  SessionStatus,
  TelegramChat,
} from "./telegram.model";
import type { CreateWatch, Watch } from "./watch.model";
import type { SearchResults } from "./promotion.model";
import type { NotificationSettings, NotificationStatus } from "./notification.model";
import type { MonitoringStatus, StartupSettings } from "./monitoring.model";
import { isAppMessage, type AppMessage } from "./app-message";

export type { CodeRequest, LoginResult, QrToken, SessionStatus } from "./telegram.model";

export const telegram = {
  monitoringStatus: () => command<MonitoringStatus>("monitoring_status"),
  saveMonitoringSettings: (enabled: boolean) =>
    command<MonitoringStatus>("save_monitoring_settings", { enabled }),
  startupSettings: () => command<StartupSettings>("startup_settings"),
  saveStartupSettings: (enabled: boolean) =>
    command<StartupSettings>("save_startup_settings", { enabled }),
  onMonitoringStatus: (callback: (status: MonitoringStatus) => void) =>
    onEvent<MonitoringStatus>("monitoring:status", callback),
  onSearchUpdated: (callback: (accountId: number) => void) =>
    onEvent<number>("search:updated", callback),
  notificationSettings: () => command<NotificationSettings>("notification_settings"),
  saveNotificationSettings: (settings: NotificationSettings) =>
    command<NotificationSettings>("save_notification_settings", { settings }),
  notificationStatus: () => command<NotificationStatus>("notification_status"),
  retryUncertainNotifications: () =>
    command<unknown>("retry_uncertain_notifications", { notificationDay: notificationDay() }).then(
      () => {},
    ),
  onNotificationStatus: (callback: (status: NotificationStatus) => void) =>
    onEvent<NotificationStatus>("notifications:status", callback),
  openPromotionLink: (url: string) =>
    command<unknown>("open_promotion_link", { url }).then(() => {}),
  searchProducts: () =>
    command<SearchResults>("search_products", { notificationDay: notificationDay() }),
  loadSearchResults: () => command<SearchResults>("load_search_results"),
  clearSearchResults: (before: number | null) =>
    command<unknown>("clear_search_results", { before }).then(() => {}),
  status: () => command<SessionStatus>("session_status"),
  getProfilePhoto: () => command<string | null>("get_profile_photo"),
  startQr: () => command<unknown>("start_qr_login").then(() => {}),
  stopQr: () => command<unknown>("stop_qr_login").then(() => {}),
  requestCode: (phone: string) => command<CodeRequest>("request_phone_code", { phone }),
  submitCode: (code: string) => command<LoginResult>("submit_phone_code", { code }),
  submitPassword: (password: string) => command<LoginResult>("submit_password", { password }),
  signOut: () => command<unknown>("sign_out").then(() => {}),
  createWatch: (input: CreateWatch) => command<Watch>("create_watch", { input }),
  updateWatch: (id: number, input: CreateWatch) => command<Watch>("update_watch", { id, input }),
  deleteWatch: (id: number) => command<unknown>("delete_watch", { id }).then(() => {}),
  listWatches: () => command<Watch[]>("list_watches"),
  listChats: () => command<TelegramChat[]>("list_chats"),
  getChatPhoto: (id: string) => command<string | null>("get_chat_photo", { id }),
  getSelectedChats: () => command<TelegramChat[]>("get_selected_chats"),
  saveSelectedChats: (chats: TelegramChat[]) =>
    command<unknown>("save_selected_chats", { chats }).then(() => {}),
  onQr: (callback: (token: QrToken) => void) => onEvent<QrToken>("telegram:qr-token", callback),
  onAuthenticated: (callback: (status: SessionStatus) => void) =>
    onEvent<SessionStatus>("telegram:auth-changed", callback),
  onPasswordRequired: (callback: (hint: string | null) => void) =>
    onEvent<string | null>("telegram:password-required", callback),
  onError: (callback: (message: AppMessage) => void) =>
    onEvent<AppMessage>("telegram:auth-error", callback),
};

export function isCodeSubmissionError(error: unknown): error is CodeSubmissionError {
  if (typeof error !== "object" || error === null) return false;

  return (
    "message" in error &&
    isAppMessage(error.message) &&
    "canRetryCode" in error &&
    typeof error.canRetryCode === "boolean"
  );
}

function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.VITE_E2E) {
    const api = testApi();

    if (!api) return Promise.reject(new Error("Tauri test API is unavailable"));

    return api.core.invoke<T>(name, args);
  }

  return invoke<T>(name, args);
}

// Keep event payload types aligned with Tauri's listen<T> API.
// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters
function onEvent<T>(name: string, callback: (payload: T) => void) {
  const api = import.meta.env.VITE_E2E ? testApi() : null;
  const subscribe = api?.event.listen ?? listen;

  return subscribe<T>(name, (event) => {
    callback(event.payload);
  });
}

function testApi() {
  return (
    window as Window & {
      __TAURI__?: { core: { invoke: typeof invoke }; event: { listen: typeof listen } };
    }
  ).__TAURI__;
}

function notificationDay(): string {
  return new Date().toLocaleDateString("pt-BR");
}
