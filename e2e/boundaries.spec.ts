import assert from "node:assert/strict";
import { $, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";

interface DesktopStatus {
  visible: boolean;
  stopping: boolean;
  suspended: boolean;
}

describe("Native command and lifecycle boundaries", () => {
  beforeEach(async () => {
    await browser.refresh();
    await command("configure", { scenario: { authorized: true, accountId: 77 } });
    await browser.execute(() => {
      localStorage.setItem("skopos.language", "en");
      window.history.replaceState({}, "", "/connected");
      (window as Window & { __SKOPOS_E2E_READY__?: boolean }).__SKOPOS_E2E_READY__ = true;
      window.dispatchEvent(new Event("skopos:e2e-ready"));
    });
    await $("h1=Products").waitForDisplayed();
  });

  it("rejects malformed bridge arguments without changing saved preferences or products", async () => {
    const products = await command("list_watches");
    const startup = await command("startup_settings");
    const notifications = await command("notification_settings");
    const cases: [string, Record<string, unknown>][] = [
      ["request_phone_code", { phone: 42 }],
      ["submit_phone_code", { code: false }],
      ["submit_password", { password: [] }],
      ["get_chat_photo", { id: 42 }],
      ["save_selected_chats", { chats: false }],
      ["create_watch", { input: false }],
      ["update_watch", { id: "invalid", input: {} }],
      ["delete_watch", { id: "invalid" }],
      ["search_products", { notificationDay: false }],
      ["clear_search_results", { before: "invalid" }],
      ["open_promotion_link", { url: false }],
      ["save_notification_settings", { settings: false }],
      ["retry_uncertain_notifications", { notificationDay: false }],
      ["save_monitoring_settings", { enabled: "false" }],
      ["save_startup_settings", { enabled: "false" }],
    ];

    for (const [name, args] of cases) {
      await assert.rejects(command(name, args), /invalid args/i, name);
    }

    assert.deepEqual(await command("list_watches"), products);
    assert.deepEqual(await command("startup_settings"), startup);
    assert.deepEqual(await command("notification_settings"), notifications);
  });

  it("reports unavailable storage without replacing existing products or preferences", async () => {
    const products = await command("list_watches");
    const startup = await command("startup_settings");
    const notifications = await command("notification_settings");
    const input = { phrases: ["Controller"], maxPriceCents: 50000, minPriceCents: null };
    const cases: [string, Record<string, unknown>, RegExp][] = [
      ["list_watches", {}, /watchStorageOpen/],
      ["create_watch", { input }, /watchStorageOpen/],
      ["update_watch", { id: 1, input }, /watchStorageOpen/],
      ["delete_watch", { id: 1 }, /watchStorageOpen/],
      ["get_selected_chats", {}, /chatStorage/],
      ["save_selected_chats", { chats: [] }, /chatStorage/],
      ["load_search_results", {}, /searchStorage/],
      ["clear_search_results", { before: null }, /searchStorage/],
      ["search_products", { notificationDay: "10/10/2026" }, /monitoringStorage/],
      ["monitoring_status", {}, /monitoringStorage/],
      ["save_monitoring_settings", { enabled: false }, /monitoringStorage/],
      ["save_startup_settings", { enabled: false }, /monitoringStorage/],
      ["notification_settings", {}, /notificationStorage/],
      ["notification_status", {}, /notificationStorage/],
      [
        "save_notification_settings",
        { settings: { telegramEnabled: false, desktopEnabled: false, language: "en" } },
        /notificationStorage/,
      ],
      ["retry_uncertain_notifications", { notificationDay: "10/10/2026" }, /notificationStorage/],
    ];

    await command("block_storage_fixture", { blocked: true });

    try {
      for (const [name, args, expected] of cases) {
        await assert.rejects(command(name, args), expected, name);
      }
    } finally {
      await command("block_storage_fixture", { blocked: false });
    }

    assert.deepEqual(await command("list_watches"), products);
    assert.deepEqual(await command("startup_settings"), startup);
    assert.deepEqual(await command("notification_settings"), notifications);
  });

  it("rejects a blank phone code with a recoverable validation error", async () => {
    await assert.rejects(command("submit_phone_code", { code: "  " }), (error: Error) => {
      assert.deepEqual(JSON.parse(error.message), {
        message: { code: "missingCode" },
        canRetryCode: true,
      });

      return true;
    });
    assert.equal((await command<{ authorized: boolean }>("session_status")).authorized, true);
  });

  it("propagates a failed session check from each account-scoped command", async () => {
    await command("configure", { scenario: { authorized: true, statusError: "DROPPED" } });
    const cases: [string, Record<string, unknown>][] = [
      ["list_chats", {}],
      ["get_chat_photo", { id: "channel:1" }],
      ["get_selected_chats", {}],
      ["save_selected_chats", { chats: [] }],
      ["load_search_results", {}],
      ["clear_search_results", { before: null }],
      ["monitoring_status", {}],
      ["save_monitoring_settings", { enabled: false }],
    ];

    for (const [name, args] of cases) {
      await assert.rejects(command(name, args), /authNetwork/, name);
    }
  });

  it("reports inaccessible databases and preserves their contents after recovery", async () => {
    const products = await command("list_watches");
    const notifications = await command("notification_settings");
    const cases: [string, [string, Record<string, unknown>, RegExp][]][] = [
      [
        "watches.sqlite",
        [
          ["list_watches", {}, /watchStorage/],
          [
            "create_watch",
            { input: { phrases: ["Controller"], maxPriceCents: null, minPriceCents: null } },
            /watchStorage/,
          ],
          [
            "update_watch",
            { id: 1, input: { phrases: ["Controller"], maxPriceCents: null, minPriceCents: null } },
            /watchStorage/,
          ],
          ["delete_watch", { id: 1 }, /watchStorage/],
          ["load_search_results", {}, /watchStorage/],
        ],
      ],
      [
        "chats.sqlite",
        [
          ["get_selected_chats", {}, /chatStorage/],
          ["save_selected_chats", { chats: [] }, /chatStorage/],
        ],
      ],
      [
        "results.sqlite",
        [
          ["load_search_results", {}, /searchStorage/],
          ["clear_search_results", { before: null }, /searchStorage/],
          ["monitoring_status", {}, /monitoringStorage/],
          ["save_monitoring_settings", { enabled: false }, /monitoringStorage/],
        ],
      ],
      [
        "notifications.sqlite",
        [
          ["notification_settings", {}, /notificationStorage/],
          ["notification_status", {}, /notificationStorage/],
          [
            "save_notification_settings",
            { settings: { telegramEnabled: false, desktopEnabled: false, language: "en" } },
            /notificationStorage/,
          ],
          [
            "retry_uncertain_notifications",
            { notificationDay: "10/10/2026" },
            /notificationStorage/,
          ],
        ],
      ],
    ];

    await command("get_selected_chats");
    await command("monitoring_status");

    for (const [database, commands] of cases) {
      await command("block_storage_fixture", { blocked: true, database });

      try {
        for (const [name, args, expected] of commands) {
          await assert.rejects(command(name, args), expected, name);
        }
      } finally {
        await command("block_storage_fixture", { blocked: false, database });
      }
    }

    assert.deepEqual(await command("list_watches"), products);
    assert.deepEqual(await command("notification_settings"), notifications);
  });

  it("revocation returns to login and rejects account-scoped reads and writes", async () => {
    await command("reject_session_fixture");
    await $("button=Phone Number").waitForDisplayed();

    assert.equal((await command<{ authorized: boolean }>("session_status")).authorized, false);
    assert.equal(
      (await command<DesktopStatus>("desktop_fixture", { action: "inspect" })).suspended,
      true,
    );

    const cases: [string, Record<string, unknown>][] = [
      ["list_chats", {}],
      ["get_chat_photo", { id: "channel:1" }],
      ["get_selected_chats", {}],
      ["save_selected_chats", { chats: [] }],
      ["load_search_results", {}],
      ["clear_search_results", { before: null }],
      ["search_products", { notificationDay: "09/10/2026" }],
      ["notification_settings", {}],
      ["notification_status", {}],
      [
        "save_notification_settings",
        { settings: { telegramEnabled: false, desktopEnabled: false, language: "en" } },
      ],
      ["retry_uncertain_notifications", { notificationDay: "09/10/2026" }],
      ["monitoring_status", {}],
      ["save_monitoring_settings", { enabled: false }],
    ];

    for (const [name, args] of cases) {
      await assert.rejects(command(name, args), /restartLogin/, name);
    }
  });

  it("closing the window keeps the tray host alive and reopening restores the same session", async () => {
    await command("desktop_fixture", { action: "close" });
    await browser.waitUntil(
      async () => !(await command<DesktopStatus>("desktop_fixture", { action: "inspect" })).visible,
    );
    const hidden = await command<DesktopStatus>("desktop_fixture", { action: "inspect" });

    assert.equal(hidden.stopping, false);
    assert.equal(hidden.suspended, false);

    assert.equal((await command<{ authorized: boolean }>("session_status")).authorized, true);

    await command("desktop_fixture", { action: "show" });
    await browser.waitUntil(
      async () => (await command<DesktopStatus>("desktop_fixture", { action: "inspect" })).visible,
    );

    await $("h1=Products").waitForDisplayed();
    assert.equal((await command<{ authorized: boolean }>("session_status")).authorized, true);
  });
});

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const serialized = await browser.execute(
    async (commandName, commandArgs) => {
      const api = (
        window as Window & {
          __TAURI__: {
            core: { invoke: (name: string, args?: Record<string, unknown>) => Promise<unknown> };
          };
        }
      ).__TAURI__;

      try {
        return JSON.stringify({ result: await api.core.invoke(commandName, commandArgs) });
      } catch (failure) {
        return JSON.stringify({ failure });
      }
    },
    name,
    args,
  );
  const response = JSON.parse(serialized) as { result: T; failure?: unknown };

  if ("failure" in response) throw new Error(JSON.stringify(response.failure));

  return response.result;
}
