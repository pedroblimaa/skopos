import assert from "node:assert/strict";
import { $, browser } from "@wdio/globals";
import { afterEach, describe, it } from "mocha";

describe("chat selection desktop flow", () => {
  afterEach(async function () {
    if (this.currentTest?.state !== "failed") return;

    console.error(
      await browser.execute(() => ({
        path: location.pathname,
        focus: document.activeElement?.outerHTML,
        content: document.body.innerHTML,
      })),
    );
    await browser.saveScreenshot("src-tauri/target/chats-e2e-failure.png");
  });

  it("navigates, saves, restores after reopening, and deselects all chats", async () => {
    await browser.refresh();
    await command("configure", { scenario: { authorized: true, accountId: 77 } });
    await command("save_selected_chats", { chats: [] });
    await browser.execute(() => {
      localStorage.setItem("skopos.language", "en");
      window.history.replaceState({}, "", "/connected");
    });
    await enableApp();
    await $("h1=Products").waitForDisplayed();
    await $("a=Chats").click();
    const group = () => $(".chats-row*=Group 1");
    await group().waitForDisplayed();
    await group().click();
    await $("button=Save selection").click();
    await $("p=Chat selection saved.").waitForDisplayed();

    assert.deepEqual(
      (await command<{ id: string }[]>("get_selected_chats")).map((chat) => chat.id),
      ["chat:1"],
    );

    await $("a=Products").click();
    await $("a=Chats").click();

    assert.equal(await group().$("input").isSelected(), true);

    await browser.refresh();
    await enableApp();
    await group().waitForDisplayed();

    assert.equal(await group().$("input").isSelected(), true);

    await group().click();
    await $("button=Save selection").click();
    await $("p=Chat selection saved.").waitForDisplayed();

    assert.deepEqual(await command("get_selected_chats"), []);
  });

  it("keeps saving controls disabled after navigating away and back", async () => {
    await browser.refresh();
    await command("configure", { scenario: { authorized: true, accountId: 78 } });
    await command("save_selected_chats", { chats: [] });
    await browser.execute(() => {
      localStorage.setItem("skopos.language", "en");
      window.history.replaceState({}, "", "/chats");
    });
    await enableApp();
    const group = () => $(".chats-row*=Group 1");
    await group().waitForDisplayed();
    await group().click();
    await pauseChatSave();

    try {
      await $("button=Save selection").click();
      await $("button=Saving…").waitForDisplayed();
      await $("a=Products").click();
      await $("h1=Products").waitForDisplayed();
      await $("a=Chats").click();
      await $("button=Saving…").waitForDisplayed();

      assert.equal(await $("button=Saving…").isEnabled(), false);
      assert.equal(await group().$("input").isEnabled(), false);
      assert.equal(await $("button[aria-label=Refresh]").isEnabled(), false);

      await finishChatSave();
      await browser.waitUntil(async () => group().$("input").isEnabled());

      assert.equal(await group().$("input").isSelected(), true);

      await group().click();
      await $(".chats-row*=Channel 2").click();
      await $("button=Save selection").click();
      await $("p=Chat selection saved.").waitForDisplayed();

      assert.deepEqual(
        (await command<{ id: string }[]>("get_selected_chats")).map((chat) => chat.id),
        ["channel:2"],
      );
    } finally {
      await finishChatSave();
    }
  });

  it("isolates accounts and preserves saved selections after invalid saves", async () => {
    await command("configure", { scenario: { authorized: true, accountId: 81, chatPhotos: true } });
    const chats = await command<{ id: string; title: string }[]>("list_chats");
    const group = chats.find((chat) => chat.id === "chat:1");
    assert.ok(group);
    await command("save_selected_chats", { chats: [group] });

    assert.equal(await command("get_chat_photo", { id: "chat:1" }), "data:image/jpeg;base64,/9g=");
    assert.equal(await command("get_chat_photo", { id: "missing" }), null);

    assert.equal(
      await commandError("save_selected_chats", { chats: [{ ...group, id: "invalid" }] }),
      "invalidChatSelection",
    );
    assert.deepEqual(await command("get_selected_chats"), [group]);

    await command("configure", { scenario: { authorized: true, accountId: 82 } });

    assert.deepEqual(await command("get_selected_chats"), []);
    assert.equal(await command("get_chat_photo", { id: "chat:1" }), null);

    await command("configure", { scenario: { authorized: true, accountId: 81, photoError: true } });

    assert.deepEqual(await command("get_selected_chats"), [group]);
    assert.equal(await commandError("get_chat_photo", { id: "chat:1" }), "chatLoadFailed");

    await command("configure", { scenario: { authorized: true, chatsError: true } });

    assert.equal(await commandError("list_chats"), "chatLoadFailed");

    await command("configure", { scenario: { authorized: false } });

    for (const name of [
      "list_chats",
      "get_selected_chats",
      "save_selected_chats",
      "get_chat_photo",
    ]) {
      assert.equal(await commandError(name, { chats: [], id: "chat:1" }), "restartLogin");
    }
  });
});

async function pauseChatSave() {
  await browser.execute(() => {
    const testWindow = window as Window & {
      __TAURI__: {
        core: { invoke: (name: string, ...args: unknown[]) => Promise<unknown> };
        event: unknown;
      };
      finishChatSave?: () => void | Promise<void>;
    };
    const originalApi = testWindow.__TAURI__;
    const originalDescriptor = Object.getOwnPropertyDescriptor(testWindow, "__TAURI__");
    if (!originalDescriptor) throw new Error("Tauri test API is missing");

    const restoreApi = () => {
      Object.defineProperty(testWindow, "__TAURI__", originalDescriptor);
      delete testWindow.finishChatSave;
    };

    testWindow.finishChatSave = restoreApi;
    Object.defineProperty(testWindow, "__TAURI__", {
      configurable: true,
      value: {
        core: {
          invoke: (name: string, ...args: unknown[]) => {
            if (name !== "save_selected_chats") return originalApi.core.invoke(name, ...args);

            let release!: () => void;
            const gate = new Promise<void>((resolve) => {
              release = resolve;
            });
            const completion = originalApi.core.invoke(name, ...args).then(async (result) => {
              await gate;
              return result;
            });

            testWindow.finishChatSave = async () => {
              restoreApi();
              release();
              await completion.catch(() => {});
            };
            return completion;
          },
        },
        event: originalApi.event,
      },
    });
  });
}

async function finishChatSave() {
  await browser.execute(async () => {
    const testWindow = window as Window & { finishChatSave?: () => void | Promise<void> };
    await testWindow.finishChatSave?.();
  });
}

async function commandError(name: string, args?: Record<string, unknown>): Promise<string> {
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
        return "unexpectedSuccess";
      } catch (error) {
        return (error as { code: string }).code;
      }
    },
    name,
    args,
  );
}

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
