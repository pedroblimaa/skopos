import assert from "node:assert/strict";
import { $, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";
import type { NotificationSettings, NotificationStatus } from "../src/notification.model";

interface SavedMessage {
  account: number;
  peer: string;
  caption: string;
  hasPhoto: boolean;
}

describe("Saved Messages notifications", () => {
  beforeEach(async () => {
    await browser.refresh();
    await telegramFixture(null);
    await command("configure_notifications_fixture", { account: 77, fixture: {}, reset: true });
    await command("clear_search_results", { before: null });
    const watches = await command<{ id: number }[]>("list_watches");

    for (const watch of watches) await command("delete_watch", { id: watch.id });

    await command("create_watch", {
      input: { phrases: ["Controle"], maxPriceCents: 50000, minPriceCents: null },
    });
    const chats = await command<unknown[]>("list_chats");

    await command("save_selected_chats", { chats });
    const settings = await command<NotificationSettings>("notification_settings");

    await command("save_notification_settings", { settings: { ...settings, language: "en" } });

    await browser.execute(() => {
      localStorage.setItem("skopos.language", "en");
      window.history.replaceState({}, "", "/connected");
      (window as Window & { __SKOPOS_E2E_READY__?: boolean }).__SKOPOS_E2E_READY__ = true;
      window.dispatchEvent(new Event("skopos:e2e-ready"));
    });
    await $("h1=Products").waitForDisplayed();
  });

  it("adds one daily divider across searches and reloads, and another on the next day", async () => {
    await command("search_products", { notificationDay: "05/10/2026" });
    await delivered();
    const first = await saved();

    assert.equal(first.length, 2);
    assert.equal(first.filter((message) => message.caption.includes("05/10/2026")).length, 1);
    assert.ok(first[0].caption.startsWith("<b>"));
    assert.equal(first[0].hasPhoto, false);
    assert.equal(first[0].caption.includes("href="), false);
    assert.ok(first.slice(1).every((message) => !message.caption.includes("05/10/2026")));
    assert.ok(first.slice(1).every((message) => message.hasPhoto));

    await browser.refresh();

    await command("configure", {
      scenario: {
        authorized: true,
        accountId: 77,
        promotionMessages: ["Controle second batch R$ 201"],
      },
    });
    await command("search_products", { notificationDay: "05/10/2026" });
    await delivered();

    assert.equal((await saved()).length, 1);
    assert.ok((await saved()).every((message) => !message.caption.includes("05/10/2026")));

    await command("configure", {
      scenario: {
        authorized: true,
        accountId: 77,
        promotionMessages: ["Controle second batch R$ 201", "Controle new day R$ 202"],
      },
    });

    await command("search_products", { notificationDay: "06/10/2026" });
    await delivered();

    assert.equal(
      (await saved()).filter((message) => message.caption.includes("06/10/2026")).length,
      1,
    );
  });

  it("does not notify offers below the automatic minimum", async () => {
    await command("configure", {
      scenario: {
        authorized: true,
        accountId: 77,
        promotionMessages: ["Controle R$ 18", "Controle R$ 201"],
      },
    });
    await command("search_products");
    await delivered();

    const messages = await saved();

    assert.equal(messages.length, 2);
    assert.ok(messages.slice(1).every((message) => message.caption.includes("R$ 201,00")));
    assert.ok(messages.every((message) => !message.caption.includes("R$ 18,00")));
    assert.equal((await desktop()).length, 1);
  });

  it("sends no divider when there are no new qualifying offers", async () => {
    await command("configure", {
      scenario: { authorized: true, accountId: 77, promotionMessages: ["Controle R$ 18"] },
    });
    await command("search_products");
    await delivered();

    assert.equal((await saved()).length, 0);
    assert.equal((await desktop()).length, 0);
  });

  it("offers two independent switches without any bot setup", async () => {
    await $(".app-profile").click();
    await $("button=Notifications").click();
    await $("input[role='switch']").waitForDisplayed();

    assert.equal(await $("input[role='switch']").isSelected(), true);
    assert.equal(await $("#notification-bot-token").isExisting(), false);

    try {
      assert.equal(await $("dialog .info-tooltip-content").isDisplayed(), false);
      assert.equal(await $("button=Close").isFocused(), true);

      const material = await browser.execute(() => {
        const dialog = document.querySelector("dialog");
        const tooltip = dialog?.querySelector(".info-tooltip-content");
        if (!dialog || !tooltip) throw new Error("Notification dialog is missing");

        return {
          containerBlur: getComputedStyle(dialog).backdropFilter,
          surfaceBlur: getComputedStyle(dialog, "::before").backdropFilter,
          tooltipBlur: getComputedStyle(tooltip).backdropFilter,
        };
      });

      assert.equal(material.containerBlur, "none");
      assert.notEqual(material.surfaceBlur, "none");
      assert.notEqual(material.tooltipBlur, "none");

      await command("focus_window");
      await browser.waitUntil(() => browser.execute(() => document.hasFocus()));
      // The embedded driver dispatches synthetic mousemove events, which cannot activate CSS :hover.
      await browser.execute(() => {
        document
          .querySelector<HTMLButtonElement>("button[aria-label='About notifications']")
          ?.focus();
      });

      assert.equal(await $("button[aria-label='About notifications']").isFocused(), true);

      await $("dialog .info-tooltip-content").waitForDisplayed();

      assert.ok((await $("dialog .info-tooltip-content").getText()).includes("Saved Messages"));

      await browser.keys("Escape");
      await $("dialog .info-tooltip-content").waitForDisplayed({ reverse: true });

      assert.equal(await $("dialog").isDisplayed(), true);
      assert.equal(await browser.execute(() => document.querySelector("dialog")?.open), true);
      assert.equal(await $("dialog").getAttribute("data-closing"), "false");
      const switchCount = await browser.execute(
        () => document.querySelectorAll("dialog input[role='switch']").length,
      );

      assert.equal(switchCount, 2);

      for (const index of [1, 2]) {
        const toggle = $(`dialog .notification-switch:nth-child(${String(index)}) input`);

        assert.equal(await toggle.isDisplayed(), true);
        assert.equal(await toggle.isEnabled(), true);
      }

      await browser.execute(() => {
        document.querySelector<HTMLButtonElement>("dialog .dialog-actions button")?.focus();
      });

      assert.equal(await $("button=Close").isFocused(), true);
      await $("dialog .info-tooltip-content").waitForDisplayed({ reverse: true });
    } catch (error) {
      await appendTooltipDiagnostics(error);
      throw error;
    }

    await browser.saveScreenshot("src-tauri/target/notification-settings-desktop.png");
    const originalSize = await browser.getWindowSize();

    try {
      await browser.setWindowSize(480, 760);
      const fits = await browser.execute(() => {
        const dialog = document.querySelector("dialog");

        return dialog !== null && dialog.scrollWidth <= dialog.clientWidth;
      });

      assert.equal(fits, true);

      await browser.saveScreenshot("src-tauri/target/notification-settings-narrow.png");
    } finally {
      await browser.setWindowSize(originalSize.width, originalSize.height);
    }

    await $("input[role='switch']").click();

    await browser.waitUntil(async () => !(await settings()).telegramEnabled);

    assert.equal((await settings()).desktopEnabled, true);

    await $("button=Close").click();
    await $("dialog").waitForExist({ reverse: true });
    await $(".app-profile").click();
    await $("button=Notifications").click();

    assert.equal(await $("input[role='switch']").isSelected(), false);
    assert.equal(await $("dialog p[role='status']").isExisting(), false);

    await $("button=Close").click();
    await $("dialog").waitForExist({ reverse: true });
  });

  it("saves compact photos once and stays quiet after cleanup and reload", async () => {
    await command("create_watch", {
      input: { phrases: ["Controle"], maxPriceCents: 50000, minPriceCents: null },
    });
    await command("search_products");
    await delivered();
    const first = await saved();

    assert.equal(first.length, 2);
    assert.equal(first[0].account, 77);
    assert.equal(first[0].peer, "self");
    assert.ok(first[1].caption.includes("Controle Ultimate Blue"));
    assert.ok(first[1].caption.includes("R$ 201,00"));
    assert.ok(first[1].caption.includes("href="));
    assert.equal(first[1].hasPhoto, true);
    assert.equal((await desktop()).length, 1);

    await command("load_search_results");

    await command("clear_search_results", { before: null });
    await command("search_products");
    await delivered();

    assert.equal((await saved()).length, 2);
    assert.equal((await desktop()).length, 1);

    await telegramFixture(null, 88);

    assert.equal((await settings()).telegramEnabled, true);
    assert.equal((await status()).pending, 0);
  });

  it("saves text when a promotion has no image", async () => {
    await telegramFixture(null, 77, false);
    await command("search_products");
    await delivered();
    const messages = await saved();

    assert.equal(messages.length, 2);
    assert.ok(messages.every((message) => !message.hasPhoto));
    assert.ok(messages.slice(1).every((message) => message.caption.includes("href=")));
  });

  it("keeps results after a definitive rejection and retries qualifying messages on another search", async () => {
    await telegramFixture("MESSAGE_EMPTY");
    await command("search_products");
    await browser.waitUntil(async () => (await status()).failure?.code === "notificationFailed");

    assert.equal((await status()).pending, 1);
    assert.equal((await command<{ matches: unknown[] }>("load_search_results")).matches.length, 2);

    await telegramFixture(null);

    await command("search_products");
    await delivered();

    assert.equal((await saved()).length, 2);
  });

  it("requires explicit retry for ambiguous saves", async () => {
    await telegramFixture("DROPPED");
    await command("search_products");
    await browser.waitUntil(async () => (await status()).uncertain === 1);
    await command("search_products");
    await browser.waitUntil(async () => (await status()).failure?.code === "notificationUncertain");

    assert.equal((await status()).uncertain, 1);
    assert.equal((await status()).pending, 1);

    await telegramFixture(null);

    await command("search_products");

    assert.equal((await status()).uncertain, 1);
    assert.equal((await saved()).length, 0);

    await command("retry_uncertain_notifications");
    await delivered();

    assert.equal((await status()).uncertain, 0);
    assert.equal((await saved()).length, 2);
  });

  it("honors Telegram cooldowns", async () => {
    await telegramFixture("FLOOD_WAIT");
    await command("search_products");
    await browser.waitUntil(async () => (await status()).failure?.code === "notificationRateLimit");
    const attempts = (await saved()).length;

    await command("search_products");

    assert.equal((await saved()).length, attempts);
    assert.equal((await status()).pending, 1);
  });

  it("leaves searches available with both channels disabled", async () => {
    await command("save_notification_settings", {
      settings: { ...(await settings()), telegramEnabled: false, desktopEnabled: false },
    });
    await command("search_products");
    await delivered();

    assert.equal((await saved()).length, 0);
    assert.equal((await desktop()).length, 0);

    await command("save_notification_settings", {
      settings: { ...(await settings()), telegramEnabled: true },
    });

    await command("search_products");
    await delivered();

    assert.equal((await saved()).length, 2);
  });

  it("replaces a definitively rejected photo with text", async () => {
    await telegramFixture("PHOTO_INVALID");
    await command("search_products");
    await delivered();
    const messages = await saved();

    assert.deepEqual(
      messages.map((message) => message.hasPhoto),
      [false, true, false],
    );
  });

  it("reports desktop failure independently and cancels saving on sign-out", async () => {
    await command("configure_notifications_fixture", {
      account: 77,
      fixture: { desktopError: true },
      reset: false,
    });
    await command("search_products");
    await delivered();
    await browser.waitUntil(async () => (await status()).failure?.code === "notificationDesktop");

    assert.equal((await saved()).length, 2);

    await command("configure_notifications_fixture", { account: 77, fixture: {}, reset: true });

    await command("search_products");
    await command("sign_out");

    await assert.rejects(command("notification_settings"), (error: Error) =>
      error.message.includes("restartLogin"),
    );

    const count = (await saved()).length;

    await browser.pause(1200);

    assert.equal((await saved()).length, count);
  });
});

async function appendTooltipDiagnostics(error: unknown) {
  if (!(error instanceof Error)) return;

  try {
    const state = await browser.execute(() => {
      const dialog = document.querySelector("dialog");
      const tooltip = dialog?.querySelector(".info-tooltip-content");
      const wrapper = tooltip?.parentElement;
      const active = document.activeElement;
      const style = tooltip && getComputedStyle(tooltip);

      return {
        active: active && {
          tag: active.tagName,
          id: active.id,
          label: active.getAttribute("aria-label"),
          text: active.textContent,
        },
        documentFocused: document.hasFocus(),
        hover: wrapper?.matches(":hover"),
        focusWithin: wrapper?.matches(":focus-within"),
        dismissed: wrapper?.getAttribute("data-dismissed"),
        visibility: style?.visibility,
        opacity: style?.opacity,
        dialogOpen: dialog?.open,
        dialogClosing: dialog?.getAttribute("data-closing"),
      };
    });

    error.message += ` Tooltip state: ${JSON.stringify(state)}`;
  } catch (diagnosticError) {
    error.message += ` Tooltip diagnostics unavailable: ${String(diagnosticError)}`;
  }
}

async function telegramFixture(error: string | null, account = 77, photos = true) {
  await command("configure", {
    scenario: {
      authorized: true,
      accountId: account,
      promotionPhotos: photos,
      savedMessageError: error,
    },
  });
}
function settings() {
  return command<NotificationSettings>("notification_settings");
}
function status() {
  return command<NotificationStatus>("notification_status");
}
async function saved() {
  return (await command<{ savedMessages: SavedMessage[] }>("inspect")).savedMessages;
}
async function desktop() {
  return (await command<{ desktop: string[] }>("inspect_notifications_fixture")).desktop;
}
async function delivered() {
  await browser.waitUntil(async () => (await status()).pending === 0, { timeout: 15000 });
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (name === "search_products" || name === "retry_uncertain_notifications") {
    args = { notificationDay: new Date().toLocaleDateString("pt-BR"), ...args };
  }

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
