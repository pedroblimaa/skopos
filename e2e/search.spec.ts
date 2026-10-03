import assert from "node:assert/strict";
import { $, $$, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";

describe("manual promotion search", () => {
  beforeEach(async () => {
    await browser.refresh();
    await command("configure", { scenario: { authorized: true, accountId: 77 } });
    await command("clear_search_results", { before: null });
    const watches = await command<{ id: number }[]>("list_watches");
    for (const watch of watches) await command("delete_watch", { id: watch.id });
    await command("create_watch", { input: { phrases: ["Controle"], maxPriceCents: 50000 } });
    await command("create_watch", { input: { phrases: ["Dishwasher"], maxPriceCents: null } });
    const chats = await command<unknown[]>("list_chats");
    await command("save_selected_chats", { chats });

    await browser.execute(() => {
      localStorage.setItem("skopos.language", "en");
      window.history.replaceState({}, "", "/connected");
    });
    await enableApp();
    await $("h1=Products").waitForDisplayed();
    await browser.waitUntil(async () => await $("button[aria-label='Search now']").isEnabled());
  });

  it("finds qualifying recent posts, expands previews, restores and deduplicates saved results", async () => {
    await $("button[aria-label='Search now']").click();
    await $(".product-matches > summary").waitForDisplayed();
    assert.equal(await $(".product-matches > summary").getText(), "Matches (2)");
    assert.equal(await $$(".product-matches").length, 1);
    assert.ok(await $(".connected-watch-name .info-tooltip[data-tone='empty']").isDisplayed());

    await $(".product-matches > summary").click();
    await $(".product-match-preview").waitForDisplayed();
    assert.equal(await $(".product-match-preview").getText(), "Controle Ultimate Blue");
    assert.equal(await $(".product-match-text").isDisplayed(), false);

    await $(".product-match-details > summary").click();
    await $(".product-match-text").waitForDisplayed();
    assert.ok((await $(".product-match-text").getText()).includes("Cupom: SAVE"));
    assert.equal(await $(".product-match-preview").isDisplayed(), false);

    await browser.refresh();
    await enableApp();
    await $(".product-matches > summary").waitForDisplayed();
    await $("button[aria-label='Search now']").click();
    await browser.waitUntil(async () => await $("button[aria-label='Search now']").isEnabled());
    assert.equal(await $(".product-matches > summary").getText(), "Matches (2)");
  });

  it("clears results through the confirmation and reports an interrupted search", async () => {
    await $("button[aria-label='Search now']").click();
    await $(".product-matches > summary").waitForDisplayed();
    await $("button[aria-label='Clear results']").click();
    await $("dialog[open]").waitForDisplayed();
    await $("dialog button[type='submit']").click();
    await browser.waitUntil(async () => !(await $("dialog[open]").isExisting()));
    assert.equal((await command<{ matches: unknown[] }>("load_search_results")).matches.length, 0);

    await command("configure", {
      scenario: { authorized: true, accountId: 77, searchError: true },
    });
    await $("button[aria-label='Search now']").click();
    await $(".connected-search-status .error").waitForDisplayed();
    assert.ok(
      (await $(".connected-search-status .error").getText()).includes("Search interrupted"),
    );
  });
});

async function enableApp() {
  await browser.execute(() => {
    (window as Window & { __SKOPOS_E2E_READY__?: boolean }).__SKOPOS_E2E_READY__ = true;
    window.dispatchEvent(new Event("skopos:e2e-ready"));
  });
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  return browser.execute(
    (commandName, commandArgs) => {
      const api = (
        window as Window & {
          __TAURI__: {
            core: { invoke: (name: string, args?: Record<string, unknown>) => Promise<unknown> };
          };
        }
      ).__TAURI__;
      return api.core.invoke(commandName, commandArgs);
    },
    name,
    args,
  ) as Promise<T>;
}
