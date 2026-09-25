import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface SessionStatus {
  authorized: boolean;
  displayName: string | null;
}

export interface QrToken {
  url: string;
  expiresAt: number;
}

export type LoginResult =
  { step: "authorized"; status: SessionStatus } | { step: "passwordRequired"; hint: string | null };

export const telegram = {
  status: () => invoke<SessionStatus>("session_status"),
  startQr: () => invoke<void>("start_qr_login"),
  stopQr: () => invoke<void>("stop_qr_login"),
  requestCode: (phone: string) =>
    invoke<{ message: string; length: number | null }>("request_phone_code", { phone }),
  submitCode: (code: string) => invoke<LoginResult>("submit_phone_code", { code }),
  submitPassword: (password: string) => invoke<LoginResult>("submit_password", { password }),
  signOut: () => invoke<void>("sign_out"),
  onQr: (callback: (token: QrToken) => void) =>
    listen<QrToken>("telegram:qr-token", (event) => callback(event.payload)),
  onAuthenticated: (callback: (status: SessionStatus) => void) =>
    listen<SessionStatus>("telegram:auth-changed", (event) => callback(event.payload)),
  onPasswordRequired: (callback: (hint: string | null) => void) =>
    listen<string | null>("telegram:password-required", (event) => callback(event.payload)),
  onError: (callback: (message: string) => void) =>
    listen<string>("telegram:auth-error", (event) => callback(event.payload)),
};

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
