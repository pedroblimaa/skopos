import assert from "node:assert/strict";
import { $, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";
import type { MonitoringStatus } from "../src/monitoring.model";

const morning = Math.floor(new Date("2026-10-07T09:00:00-03:00").getTime() / 1000);
const evening = morning + 9 * 3600;

describe("Automatic monitoring", () => {
  beforeEach(async () => {
    await browser.refresh();
    await scenario(morning, 100, 200);
    await command("configure_notifications_fixture", { account: 77, fixture: {}, reset: true });
    await command("clear_search_results", { before: null });

    for (const watch of await command<{ id: number }[]>("list_watches")) {
      await command("delete_watch", { id: watch.id });
    }
    await command("create_watch", {
      input: { phrases: ["Controle"], maxPriceCents: 50000, minPriceCents: null },
    });
    const chats = await command<unknown[]>("list_chats");

    await command("save_selected_chats", { chats: chats.slice(0, 1) });
    await command("configure_monitoring_fixture", { now: morning, reset: true });
    await browser.execute(() => {
      localStorage.setItem("skopos.language", "en");
      window.history.replaceState({}, "", "/connected");
      (window as Window & { __SKOPOS_E2E_READY__?: boolean }).__SKOPOS_E2E_READY__ = true;
      window.dispatchEvent(new Event("skopos:e2e-ready"));
    });
    await $("h1=Products").waitForDisplayed();
  });

  it("persists daily slots through reload and cleanup, then checks new messages at 18:00", async () => {
    const first = await tick();

    assert.equal(first.lastAttempt, morning);
    assert.equal((await results()).matches.length, 1);

    await browser.refresh();

    await command("clear_search_results", { before: null });

    assert.equal((await tick()).lastAttempt, morning);
    assert.equal((await results()).matches.length, 0);

    await scenario(evening, 101, 210);

    await command("configure_monitoring_fixture", { now: evening, reset: false });

    assert.equal((await tick()).lastAttempt, evening);
    assert.equal((await results()).matches.length, 1);

    await command("clear_search_results", { before: null });

    assert.equal((await tick()).lastAttempt, evening);
    assert.equal((await results()).matches.length, 0);
  });

  it("leaves unconfigured slots available and runs the background worker when configuration is ready", async () => {
    for (const watch of await command<{ id: number }[]>("list_watches")) {
      await command("delete_watch", { id: watch.id });
    }
    await command("save_selected_chats", { chats: [] });

    assert.equal((await tick()).lastAttempt, null);

    await command("create_watch", {
      input: { phrases: ["Controle"], maxPriceCents: 50000, minPriceCents: null },
    });

    assert.equal((await tick()).lastAttempt, null);

    const chats = await command<unknown[]>("list_chats");
    await command("save_selected_chats", { chats: chats.slice(0, 1) });
    await command("wake_monitoring_fixture");
    await browser.waitUntil(async () => {
      const status = await command<MonitoringStatus>("monitoring_status");

      return status.lastAttempt === morning && !status.isRunning;
    });

    assert.equal((await results()).matches.length, 1);
    assert.equal((await command<MonitoringStatus>("monitoring_status")).failure, null);
  });

  it("records failed automatic attempts and does not retry their daily slot", async () => {
    await command("configure", {
      scenario: { authorized: true, accountId: 77, historyNow: morning, searchError: true },
    });
    await command("configure_monitoring_fixture", { now: morning, reset: true });

    const failed = await tick();

    assert.equal(failed.lastAttempt, morning);
    assert.ok(failed.failure);
    assert.equal(failed.isRunning, false);
    assert.equal((await results()).matches.length, 0);

    await scenario(morning + 60, 100, 200);
    await command("configure_monitoring_fixture", { now: morning + 60, reset: false });

    assert.equal((await tick()).lastAttempt, morning);
    assert.equal((await results()).matches.length, 0);

    await scenario(evening, 101, 210);
    await command("configure_monitoring_fixture", { now: evening, reset: false });

    assert.equal((await tick()).lastAttempt, evening);
    assert.equal((await results()).matches.length, 1);
  });

  it("keeps monitoring and Windows startup independent and runs once after a late first launch", async () => {
    await command("configure_monitoring_fixture", { now: evening, reset: true });
    await scenario(evening, 100, 200);
    await command("configure_monitoring_fixture", { now: evening, reset: false });

    assert.equal((await tick()).lastAttempt, evening);

    await command("clear_search_results", { before: null });

    assert.equal((await tick()).lastAttempt, evening);
    assert.equal((await results()).matches.length, 0);

    await $(".app-profile").click();
    await $("button=Monitoring").click();
    await $("input[role='switch']").waitForDisplayed();
    await browser.saveScreenshot("src-tauri/target/monitoring-settings-desktop.png");
    const switches = browser.$$("input[role='switch']");

    assert.equal(await switches[0].isSelected(), true);
    assert.equal(await switches[1].isSelected(), true);

    await switches[0].click();

    await browser.waitUntil(
      async () => !(await command<MonitoringStatus>("monitoring_status")).enabled,
    );

    assert.equal((await command<{ enabled: boolean }>("startup_settings")).enabled, true);

    await $("button=Close").click();
  });
});

async function scenario(now: number, id: number, price: number) {
  await command("configure", {
    scenario: {
      authorized: true,
      accountId: 77,
      historyNow: now,
      historyFirstId: id,
      promotionMessages: [`Controle R$ ${String(price)} https://shop.example/offer`],
    },
  });
}
function tick() {
  return command<MonitoringStatus>("tick_monitoring_fixture");
}
function results() {
  return command<{ matches: unknown[] }>("load_search_results");
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
