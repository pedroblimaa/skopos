import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CodeRequest,
  CodeSubmissionError,
  LoginResult,
  QrToken,
  SessionStatus,
} from "./telegram.model";
import type { CreateWatch, Watch } from "./watch.model";
import { isAppMessage, type AppMessage } from "./app-message";

export type { CodeRequest, LoginResult, QrToken, SessionStatus } from "./telegram.model";

export const telegram = {
  status: () => command<SessionStatus>("session_status"),
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
