import { beforeEach, describe, expect, it, vi } from "vitest";

const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: api.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));

import { isCodeSubmissionError, telegram } from "./telegram";

beforeEach(() => {
  vi.clearAllMocks();
  api.invoke.mockResolvedValue(null);
});

describe("Telegram IPC adapter", () => {
  it("uses notification commands and typed progress events", async () => {
    const settings = {
      telegramEnabled: true,
      desktopEnabled: true,
      language: "en",
    };
    await telegram.notificationSettings();
    await telegram.saveNotificationSettings(settings);
    await telegram.notificationStatus();
    await telegram.retryUncertainNotifications();

    expect(api.invoke.mock.calls).toEqual([
      ["notification_settings", undefined],
      ["save_notification_settings", { settings }],
      ["notification_status", undefined],
      [
        "retry_uncertain_notifications",
        { notificationDay: new Date().toLocaleDateString("pt-BR") },
      ],
    ]);
    const callback = vi.fn();
    const delivery = { pending: 1, uncertain: 0, failure: null };
    api.listen.mockImplementationOnce(
      (_name: string, receive: (event: { payload: typeof delivery }) => void) => {
        receive({ payload: delivery });
        return Promise.resolve(() => {});
      },
    );
    await telegram.onNotificationStatus(callback);
    expect(callback).toHaveBeenCalledWith(delivery);
  });
  it("uses typed search commands and preserves cleanup cutoffs", async () => {
    await telegram.searchProducts();
    await telegram.loadSearchResults();
    await telegram.clearSearchResults(null);
    await telegram.clearSearchResults(150);

    expect(api.invoke.mock.calls).toEqual([
      ["search_products", { notificationDay: new Date().toLocaleDateString("pt-BR") }],
      ["load_search_results", undefined],
      ["clear_search_results", { before: null }],
      ["clear_search_results", { before: 150 }],
    ]);
  });
  it("preserves opaque chat IDs and selection metadata across the bridge", async () => {
    const chat = {
      id: "channel:9007199254740993",
      title: "Deals",
      kind: "channel" as const,
      username: "deals",
      available: true,
    };
    api.invoke.mockResolvedValue([chat]);

    expect(await telegram.listChats()).toEqual([chat]);
    expect(await telegram.getSelectedChats()).toEqual([chat]);
    await telegram.saveSelectedChats([chat]);
    await telegram.getChatPhoto(chat.id);

    expect(api.invoke.mock.calls).toEqual([
      ["list_chats", undefined],
      ["get_selected_chats", undefined],
      ["save_selected_chats", { chats: [chat] }],
      ["get_chat_photo", { id: chat.id }],
    ]);
  });
  it("uses the expected Tauri commands and arguments", async () => {
    await telegram.status();
    await telegram.getProfilePhoto();
    await telegram.startQr();
    await telegram.stopQr();
    await telegram.requestCode("+5511999999999");
    await telegram.submitCode("12345");
    await telegram.submitPassword("secret");
    await telegram.signOut();
    await telegram.createWatch({ phrases: ["RTX 5070"], maxPriceCents: null, minPriceCents: null });
    await telegram.listWatches();
    await telegram.updateWatch(7, {
      phrases: ["RTX 5080"],
      maxPriceCents: 400000,
      minPriceCents: null,
    });
    await telegram.deleteWatch(7);

    expect(api.invoke.mock.calls).toEqual([
      ["session_status", undefined],
      ["get_profile_photo", undefined],
      ["start_qr_login", undefined],
      ["stop_qr_login", undefined],
      ["request_phone_code", { phone: "+5511999999999" }],
      ["submit_phone_code", { code: "12345" }],
      ["submit_password", { password: "secret" }],
      ["sign_out", undefined],
      [
        "create_watch",
        { input: { phrases: ["RTX 5070"], maxPriceCents: null, minPriceCents: null } },
      ],
      ["list_watches", undefined],
      [
        "update_watch",
        { id: 7, input: { phrases: ["RTX 5080"], maxPriceCents: 400000, minPriceCents: null } },
      ],
      ["delete_watch", { id: 7 }],
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

  it("recognizes structured code errors without losing retry metadata", () => {
    expect(isCodeSubmissionError({ message: { code: "invalidCode" }, canRetryCode: true })).toBe(
      true,
    );
    expect(isCodeSubmissionError({ message: { code: "codeExpired" }, canRetryCode: false })).toBe(
      true,
    );
    expect(isCodeSubmissionError({ message: "invalid", canRetryCode: true })).toBe(false);
    expect(isCodeSubmissionError({ message: { code: "invalidCode" } })).toBe(false);
    expect(isCodeSubmissionError(null)).toBe(false);
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

  it("passes promotion links to the native browser opener", async () => {
    api.invoke.mockResolvedValue(undefined);
    await telegram.openPromotionLink("https://shop.example/item");
    expect(api.invoke).toHaveBeenCalledWith("open_promotion_link", {
      url: "https://shop.example/item",
    });
  });
});
