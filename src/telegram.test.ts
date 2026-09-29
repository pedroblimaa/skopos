import { beforeEach, describe, expect, it, vi } from "vitest";

const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: api.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));

import { errorMessage, isCodeSubmissionError, telegram } from "./telegram";

beforeEach(() => {
  vi.clearAllMocks();
  api.invoke.mockResolvedValue(null);
});

describe("Telegram IPC adapter", () => {
  it("uses the expected Tauri commands and arguments", async () => {
    await telegram.status();
    await telegram.startQr();
    await telegram.stopQr();
    await telegram.requestCode("+5511999999999");
    await telegram.submitCode("12345");
    await telegram.submitPassword("secret");
    await telegram.signOut();

    expect(api.invoke.mock.calls).toEqual([
      ["session_status", undefined],
      ["start_qr_login", undefined],
      ["stop_qr_login", undefined],
      ["request_phone_code", { phone: "+5511999999999" }],
      ["submit_phone_code", { code: "12345" }],
      ["submit_password", { password: "secret" }],
      ["sign_out", undefined],
    ]);
  });

  it("unwraps Tauri event payloads", async () => {
    api.listen.mockImplementation(
      (_name: string, callback: (event: { payload: unknown }) => void) => {
        callback({ payload: { value: "received" } });
        return Promise.resolve(() => {});
      },
    );
    const callback = vi.fn();
    await telegram.onQr(callback);
    await telegram.onAuthenticated(callback);
    await telegram.onPasswordRequired(callback);
    await telegram.onError(callback);
    expect(api.listen.mock.calls.map((call: unknown[]) => call[0])).toEqual([
      "telegram:qr-token",
      "telegram:auth-changed",
      "telegram:password-required",
      "telegram:auth-error",
    ]);
    expect(callback).toHaveBeenCalledTimes(4);
    expect(callback).toHaveBeenCalledWith({ value: "received" });
  });

  it("normalizes unknown errors", () => {
    expect(errorMessage(new Error("failed"))).toBe("failed");
    expect(errorMessage("failed")).toBe("failed");
    expect(errorMessage(42)).toBe("42");
    expect(errorMessage({ message: "expired", canRetryCode: false })).toBe("expired");
    expect(isCodeSubmissionError({ message: "invalid", canRetryCode: true })).toBe(true);
    expect(isCodeSubmissionError({ message: "invalid" })).toBe(false);
  });

  it("uses the interceptable global API in the E2E build", async () => {
    vi.stubEnv("VITE_E2E", "1");
    const testInvoke = vi.fn().mockResolvedValue({ authorized: false, displayName: null });
    const testListen = vi.fn().mockResolvedValue(() => {});
    Object.assign(window, {
      __TAURI__: { core: { invoke: testInvoke }, event: { listen: testListen } },
    });
    try {
      await telegram.status();
      await telegram.onError(() => {});
      expect(testInvoke).toHaveBeenCalledWith("session_status", undefined);
      expect(testListen).toHaveBeenCalledWith("telegram:auth-error", expect.any(Function));
      expect(api.invoke).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
      Reflect.deleteProperty(window, "__TAURI__");
    }
  });

  it("fails clearly if the E2E API is missing", async () => {
    vi.stubEnv("VITE_E2E", "1");
    try {
      await expect(telegram.status()).rejects.toThrow("Tauri test API is unavailable");
    } finally {
      vi.unstubAllEnvs();
    }
  });
});
