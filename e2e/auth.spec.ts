import assert from "node:assert/strict";
import { $, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";

interface Scenario {
  authorized?: boolean;
  passwordRequired?: boolean;
  codeExpired?: boolean;
  passwordError?: boolean;
  immediateAuthorized?: boolean;
  qrFailures?: number;
  requestError?: string;
  statusError?: string;
  statusErrorAfterLogin?: boolean;
  signOutError?: string;
}

describe("Telegram desktop flows", () => {
  let first = true;

  beforeEach(async () => {
    if (!first) await browser.refresh();

    first = false;

    await browser.execute(() => {
      localStorage.setItem("skopos.language", "en");
      window.history.replaceState({}, "", "/");
    });
  });

  it("shows and refreshes a QR token, then completes QR login", async () => {
    await setUp();

    await $("h1=Authorize Telegram").waitForDisplayed();
    await browser.waitUntil(async () => (await $(".qr-expiry").getText()).startsWith("Expires in"));

    await command("refresh_qr");
    await browser.waitUntil(async () => /Expires in 01:/.test(await $(".qr-expiry").getText()));

    await command("authorize_qr");
    await $("h1=Products").waitForDisplayed();
  });

  it("signs in with a phone code", async () => {
    await setUp();

    await choosePhone();
    await $("button=Send login code").click();

    await $("#verification-code").setValue("12345");
    await $("button=Verify code").click();

    await $("h1=Products").waitForDisplayed();
  });

  it("handles the password challenge", async () => {
    await setUp({ passwordRequired: true });

    await choosePhone();
    await $("button=Send login code").click();

    await $("#verification-code").setValue("12345");
    await $("button=Verify code").click();

    await $("#password").waitForDisplayed();

    await $("#password").setValue("secret");
    await $("button=Continue").click();
    await $("h1=Products").waitForDisplayed();
  });

  it("handles a password challenge after QR login", async () => {
    await setUp();

    await browser.waitUntil(async () => (await $(".qr-expiry").getText()).startsWith("Expires in"));

    await command("require_qr_password");
    await $("#password").waitForDisplayed();

    await $("#password").setValue("secret");
    await $("button=Continue").click();
    await $("h1=Products").waitForDisplayed();
  });

  it("returns to phone entry when changing the number", async () => {
    await setUp();

    await choosePhone();
    await $("button=Send login code").click();

    await $("#verification-code").waitForDisplayed();

    await $("button=Change phone number").click();
    await $("#phone").waitForDisplayed();
  });

  it("connects when the phone request authorizes without a code", async () => {
    await setUp({ immediateAuthorized: true });

    await choosePhone();
    await $("button=Send login code").click();

    await $("h1=Products").waitForDisplayed();
    await $("strong=Pedro").waitForDisplayed();
  });

  it("restores a session and signs out from the profile menu", async () => {
    await setUp({ authorized: true });

    await $("strong=Pedro").waitForDisplayed();

    await $(".app-profile").click();
    await $("button=Sign out").click();
    await $("h1=Authorize Telegram").waitForDisplayed();
  });

  it("opens Products through a loading screen without flashing login for a saved session", async () => {
    await browser.execute(() => {
      const startup = { sawLoading: false, sawLogin: false };
      const observer = new MutationObserver((records) => {
        for (const record of records) {
          for (const node of Array.from(record.addedNodes)) {
            if (!(node instanceof Element)) continue;

            if (node.matches(".session-loading") || node.querySelector(".session-loading")) {
              startup.sawLoading = true;
            }

            if (node.matches(".auth-heading") || node.querySelector(".auth-heading")) {
              startup.sawLogin = true;
            }
          }
        }
      });

      observer.observe(document.body, { childList: true, subtree: true });
      Object.assign(window, { startupObservation: startup, startupObserver: observer });
    });

    await setUp({ authorized: true });
    await $("h1=Products").waitForDisplayed();

    const observation = await browser.execute(() => {
      const observed = window as Window & {
        startupObservation?: { sawLoading: boolean; sawLogin: boolean };
        startupObserver?: MutationObserver;
      };
      if (!observed.startupObserver || !observed.startupObservation) {
        throw new Error("Startup observation is missing");
      }

      observed.startupObserver.disconnect();

      return observed.startupObservation;
    });

    assert.equal(observation.sawLoading, true);
    assert.equal(observation.sawLogin, false);
    assert.equal(await $(".qr-frame").isExisting(), false);
  });

  it("shows a failed session lookup and still offers login", async () => {
    await setUp({ statusError: "Session unavailable" });

    await $("[role=alert]").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "Could not reach Telegram. Check your connection and try again.",
    );
    await $("h1=Authorize Telegram").waitForDisplayed();
  });

  it("redirects an unauthorized connected route to login", async () => {
    await setUp();

    await $("h1=Authorize Telegram").waitForDisplayed();

    await browser.execute(() => {
      window.history.pushState({}, "", "/connected");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });

    await $("h1=Authorize Telegram").waitForDisplayed();
    await browser.waitUntil(async () => (await browser.getUrl()).endsWith("/login"));
  });

  it("shows a QR failure and allows retry", async () => {
    await setUp({ qrFailures: 1 });

    await $("[role=alert]").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "Could not reach Telegram. Check your connection and try again.",
    );

    await $("button=Try again").click();
    await browser.waitUntil(async () => (await $(".qr-expiry").getText()).startsWith("Expires in"));

    const counts = await command<{ qrStarts: number }>("inspect");
    assert.equal(counts.qrStarts, 2);
  });

  it("cancels QR login when switching to phone", async () => {
    await setUp();

    await browser.waitUntil(async () => (await $(".qr-expiry").getText()).startsWith("Expires in"));

    await $("button=Phone Number").click();
    await browser.waitUntil(
      async () => !(await command<{ qrActive: boolean }>("inspect")).qrActive,
    );
  });

  it("shows a Telegram event error", async () => {
    await setUp();

    await browser.waitUntil(async () => (await $(".qr-expiry").getText()).startsWith("Expires in"));

    await command("fail_qr");
    await $("[role=alert]").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "Could not reach Telegram. Check your connection and try again.",
    );
  });

  it("shows a phone code request error", async () => {
    await setUp({ requestError: "Could not send code" });

    await choosePhone();
    await $("button=Send login code").click();

    await $("[role=alert]").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "Could not reach Telegram. Check your connection and try again.",
    );
  });

  it("keeps the code step after an invalid code", async () => {
    await setUp();

    await choosePhone();
    await $("button=Send login code").click();

    await $("#verification-code").setValue("00000");
    await $("button=Verify code").click();

    await $("[role=alert]").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "That verification code is invalid. Check it and try again.",
    );
    await $("#verification-code").waitForDisplayed();
  });

  it("requests a new code after the previous code expires", async () => {
    await setUp({ codeExpired: true });

    await choosePhone();
    await $("button=Send login code").click();
    await $("#verification-code").setValue("12345");
    await $("button=Verify code").click();

    await $("#phone").waitForDisplayed();
    assert.equal(await $("[role=alert]").getText(), "This code expired. Request a new one.");
    await $("button=Send login code").click();
    await $("#verification-code").waitForDisplayed();
  });

  it("leaves code entry when session verification fails after login", async () => {
    await setUp({ statusErrorAfterLogin: true });

    await choosePhone();
    await $("button=Send login code").click();
    await $("#verification-code").setValue("12345");
    await $("button=Verify code").click();

    await $("#phone").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "Could not reach Telegram. Check your connection and try again.",
    );
  });

  it("keeps the password step after an incorrect password", async () => {
    await setUp({ passwordRequired: true });

    await choosePhone();
    await $("button=Send login code").click();

    await $("#verification-code").setValue("12345");
    await $("button=Verify code").click();

    await $("#password").setValue("wrong");
    await $("button=Continue").click();
    await $("[role=alert]").waitForDisplayed();
    assert.equal(await $("[role=alert]").getText(), "That password is incorrect. Try again.");
    await $("#password").waitForDisplayed();
  });

  it("restarts phone login after a password request fails", async () => {
    await setUp({ passwordRequired: true, passwordError: true });

    await choosePhone();
    await $("button=Send login code").click();
    await $("#verification-code").setValue("12345");
    await $("button=Verify code").click();
    await $("#password").setValue("secret");
    await $("button=Continue").click();

    await $("#phone").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "Could not reach Telegram. Check your connection and try again.",
    );
    await $("button=Send login code").click();
    await $("#verification-code").waitForDisplayed();
  });

  it("keeps Products after a sign-out failure", async () => {
    await setUp({ authorized: true, signOutError: "Could not disconnect" });

    await $(".app-profile").click();
    await $("button=Sign out").click();
    await $("[role=alert]").waitForDisplayed();
    assert.equal(
      await $("[role=alert]").getText(),
      "Could not reach Telegram. Check your connection and try again.",
    );
    await $("h1=Products").waitForDisplayed();
  });
});

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

async function setUp(scenario: Scenario = {}) {
  await command("configure", { scenario });

  await browser.execute(() => {
    (window as Window & { __SKOPOS_E2E_READY__?: boolean }).__SKOPOS_E2E_READY__ = true;
    window.dispatchEvent(new Event("skopos:e2e-ready"));
  });
}

async function choosePhone() {
  await $("button=Phone Number").click();
  await $("#phone").setValue("+5511999999999");
}
