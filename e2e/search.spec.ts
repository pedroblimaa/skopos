import assert from "node:assert/strict";
import { $, $$, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";
import type { SearchResults } from "../src/promotion.model";

describe("manual promotion search", () => {
  beforeEach(async () => {
    await browser.refresh();
    await command("configure", {
      scenario: { authorized: true, accountId: 77, promotionPhotos: true },
    });
    await command("clear_search_results", { before: null });
    const watches = await command<{ id: number }[]>("list_watches");
    for (const watch of watches) await command("delete_watch", { id: watch.id });
    await command("create_watch", {
      input: { phrases: ["Controle"], maxPriceCents: 50000, minPriceCents: null },
    });
    await command("create_watch", {
      input: { phrases: ["Dishwasher"], maxPriceCents: null, minPriceCents: null },
    });
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

  it("filters cheap accessories then rematches saved results after editing the minimum", async () => {
    const product = await command<{ id: number }>("create_watch", {
      input: { phrases: ["Dishwasher"], maxPriceCents: 500000, minPriceCents: 0 },
    });
    await command("configure", {
      scenario: {
        authorized: true,
        accountId: 77,
        promotionMessages: ["Detergent for Dishwasher R$ 18", "Dishwasher R$ 1.526,88"],
      },
    });
    const all = await command<SearchResults>("search_products");
    const original = all.matches.filter((match) => match.watchId === product.id);
    assert.ok(original.some((match) => match.priceCents === 1800));
    assert.ok(original.some((match) => match.priceCents === 152688));

    await command("update_watch", {
      id: product.id,
      input: { phrases: ["Dishwasher"], maxPriceCents: 500000, minPriceCents: null },
    });
    const filtered = await command<SearchResults>("load_search_results");
    const automatic = filtered.matches.filter((match) => match.watchId === product.id);
    assert.ok(automatic.length > 0);
    assert.ok(automatic.every((match) => match.priceCents === 152688));

    await command("update_watch", {
      id: product.id,
      input: { phrases: ["Dishwasher"], maxPriceCents: 500000, minPriceCents: 0 },
    });
    const restored = await command<SearchResults>("load_search_results");
    assert.equal(
      restored.matches.filter((match) => match.watchId === product.id).length,
      original.length,
    );
  });

  it("rejects search without products or chats and rejects an invalid cleanup cutoff", async () => {
    assert.deepEqual(await commandError("clear_search_results", { before: -1 }), {
      code: "invalidSearchCutoff",
    });

    await command("save_selected_chats", { chats: [] });
    assert.deepEqual(await commandError("search_products"), { code: "searchNeedsChats" });

    const watches = await command<{ id: number }[]>("list_watches");
    for (const watch of watches) await command("delete_watch", { id: watch.id });

    assert.deepEqual(await commandError("search_products"), { code: "searchNeedsProducts" });
  });

  it("validates product criteria and promotion links at the native boundary", async () => {
    const watches = await command<{ id: number }[]>("list_watches");
    const input = { phrases: [], maxPriceCents: null, minPriceCents: null };

    assert.deepEqual(await commandError("create_watch", { input }), { code: "invalidPhrase" });
    assert.deepEqual(await commandError("update_watch", { id: watches[0].id, input }), {
      code: "invalidPhrase",
    });

    for (const url of ["javascript:alert(1)", "file:///example", "tg://resolve?domain=example"]) {
      assert.deepEqual(await commandError("open_promotion_link", { url }), {
        code: "openLinkFailed",
      });
    }
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
    await $(".product-match-image").waitForDisplayed();
    await browser.waitUntil(
      async () =>
        await browser.execute(() => {
          const image = document.querySelector<HTMLImageElement>(".product-match-image");
          return image !== null && image.complete && image.naturalWidth > 0;
        }),
    );
    assert.equal(
      await $(".product-match-text a").getAttribute("href"),
      "https://shop.example/item",
    );

    await browser.refresh();
    await enableApp();
    await $(".product-matches > summary").waitForDisplayed();
    await $(".product-matches > summary").click();
    await $(".product-match-details > summary").click();
    await $(".product-match-image").waitForDisplayed();
    await $("button[aria-label='Search now']").click();
    await browser.waitUntil(async () => await $("button[aria-label='Search now']").isEnabled());
    assert.equal(await $(".product-matches > summary").getText(), "Matches (2)");
  });

  it("reuses local results across navigation and opens message links without toggling the promotion", async () => {
    await recordCalls();
    await $("button[aria-label='Search now']").click();
    await $(".product-matches > summary").waitForDisplayed();
    await $("a=Chats").click();
    await $("h1=Chats").waitForDisplayed();

    await $("a=Products").click();
    await $("h1=Products").waitForDisplayed();
    assert.equal(await $(".product-matches > summary").getText(), "Matches (2)");
    const calls = await readCalls();
    assert.equal(
      calls.filter((call) => call.name === "load_search_results" || call.name === "list_watches")
        .length,
      0,
    );
    assert.equal(await $$(".connected-search-status .error").length, 0);

    await $(".product-matches > summary").click();
    await $(".product-match-details > summary").click();
    await $(".product-match-text a").click();
    const opened = (await readCalls()).find((call) => call.name === "open_promotion_link");
    assert.deepEqual(opened?.args, { url: "https://shop.example/item" });
    assert.equal(await $(".product-match-text").isDisplayed(), true);
  });

  it("keeps matches usable when Telegram photo downloads fail", async () => {
    await command("configure", {
      scenario: { authorized: true, accountId: 77, promotionPhotos: true, photoError: true },
    });
    await $("button[aria-label='Search now']").click();
    await $(".product-matches > summary").waitForDisplayed();
    await $(".product-matches > summary").click();
    await $(".product-match-details > summary").click();
    await $(".product-match-text").waitForDisplayed();
    assert.equal(await $$(".product-match-image").length, 0);
    assert.equal(await $$(".connected-search-status .error").length, 0);
  });

  it("shares a qualifying offer across overlapping watches with separate ceilings", async () => {
    const broad = await command<{ id: number }>("create_watch", {
      input: { phrases: ["RTX 5070"], maxPriceCents: 400000, minPriceCents: null },
    });
    const specific = await command<{ id: number }>("create_watch", {
      input: { phrases: ["RTX 5070 ASUS"], maxPriceCents: 400000, minPriceCents: null },
    });
    const cheaper = await command<{ id: number }>("create_watch", {
      input: { phrases: ["RTX 5070"], maxPriceCents: 380000, minPriceCents: null },
    });
    await command("configure", {
      scenario: { authorized: true, accountId: 77, promotionMessages: ["RTX 5070 ASUS R$ 3.900"] },
    });
    await browser.refresh();
    await enableApp();
    await browser.waitUntil(async () => await $("button[aria-label='Search now']").isEnabled());

    await $("button[aria-label='Search now']").click();
    await $(".product-matches > summary").waitForDisplayed();
    await browser.waitUntil(async () => await $("button[aria-label='Search now']").isEnabled());
    const results = await command<SearchResults>("load_search_results");

    assert.equal(results.matches.filter((match) => match.watchId === broad.id).length, 2);
    assert.equal(results.matches.filter((match) => match.watchId === specific.id).length, 2);
    assert.equal(results.matches.filter((match) => match.watchId === cheaper.id).length, 0);
    assert.ok(results.matches.every((match) => match.priceCents === 390000));
    assert.equal(await $$(".product-matches").length, 2);

    await browser.refresh();
    await enableApp();
    await $(".product-matches > summary").waitForDisplayed();

    assert.equal(await $$(".product-matches").length, 2);
  });

  it("rejects over-budget offers with cheap shipping or installments and shows the full qualifying price", async () => {
    await command("configure", {
      scenario: {
        authorized: true,
        accountId: 77,
        promotionMessages: [
          "Controle R$ 600 + frete R$ 20",
          "Controle R$ 600 ou R$ 60 em 10x",
          "Controle R$ 400 + frete R$ 20 ou R$ 40 em 10x",
        ],
      },
    });

    await $("button[aria-label='Search now']").click();
    await $(".product-matches > summary").waitForDisplayed();
    await browser.waitUntil(async () => await $("button[aria-label='Search now']").isEnabled());
    const results = await command<SearchResults>("load_search_results");

    assert.equal(results.matches.length, 2);
    assert.ok(results.matches.every((match) => match.priceCents === 40000));
    assert.ok(results.matches.every((match) => match.message.messageId === 98));

    await $(".product-matches > summary").click();

    assert.match(await $(".product-match-price").getText(), /400,00/);
  });

  it("clears results through the confirmation and reports an interrupted search", async () => {
    await $("button[aria-label='Search now']").click();
    await $(".product-matches > summary").waitForDisplayed();
    await $("button[aria-label='Clear results']").click();
    await $("dialog[open]").waitForDisplayed();
    assert.equal(
      await $("dialog")
        .getText()
        .then((text) => text.includes("Remove saved matches.")),
      false,
    );
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

interface RecordedCall {
  name: string;
  args?: Record<string, unknown>;
}
async function recordCalls() {
  await browser.execute(() => {
    const testWindow = window as unknown as Window & {
      __TAURI__: {
        core: { invoke: (name: string, args?: Record<string, unknown>) => Promise<unknown> };
        event: unknown;
      };
      promotionCalls: { name: string; args?: Record<string, unknown> }[];
    };
    const originalApi = testWindow.__TAURI__;
    testWindow.promotionCalls = [];
    Object.defineProperty(testWindow, "__TAURI__", {
      configurable: true,
      value: {
        core: {
          invoke: (name: string, args?: Record<string, unknown>) => {
            testWindow.promotionCalls.push({ name, args });
            if (name === "open_promotion_link") return Promise.resolve();
            return originalApi.core.invoke(name, args);
          },
        },
        event: originalApi.event,
      },
    });
  });
}
async function readCalls(): Promise<RecordedCall[]> {
  return browser.execute(
    () => (window as Window & { promotionCalls?: RecordedCall[] }).promotionCalls ?? [],
  );
}

async function enableApp() {
  await browser.execute(() => {
    (window as Window & { __SKOPOS_E2E_READY__?: boolean }).__SKOPOS_E2E_READY__ = true;
    window.dispatchEvent(new Event("skopos:e2e-ready"));
  });
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (name === "search_products" || name === "retry_uncertain_notifications") {
    args = { notificationDay: new Date().toLocaleDateString("pt-BR"), ...args };
  }
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

async function commandError(name: string, args?: Record<string, unknown>): Promise<unknown> {
  if (name === "search_products") {
    args = { notificationDay: new Date().toLocaleDateString("pt-BR"), ...args };
  }
  return browser.execute(
    async (commandName, commandArgs) => {
      const api = (
        window as Window & {
          __TAURI__: {
            core: { invoke: (name: string, args?: Record<string, unknown>) => Promise<unknown> };
          };
        }
      ).__TAURI__;

      try {
        await api.core.invoke(commandName, commandArgs);
        return null;
      } catch (error) {
        return error;
      }
    },
    name,
    args,
  );
}
