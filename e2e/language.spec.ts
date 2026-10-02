import assert from "node:assert/strict";
import { $, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";

describe("desktop language and typography", () => {
  beforeEach(async () => {
    await browser.refresh();
    await command("configure", { scenario: { authorized: true } });
    await browser.execute(() => {
      localStorage.clear();
      window.history.replaceState({}, "", "/connected");
    });
  });

  for (const [locale, heading] of [
    ["pt-PT", "Produtos"],
    ["en-US", "Products"],
    ["es-ES", "Products"],
  ]) {
    it(`starts in the expected language for ${locale}`, async () => {
      await browser.execute((systemLanguage) => {
        Object.defineProperty(navigator, "language", { configurable: true, value: systemLanguage });
      }, locale);

      await enableApp();
      await $(`h1=${heading}`).waitForDisplayed();

      const preference = await browser.execute(() => localStorage.getItem("skopos.language"));
      assert.equal(preference, null);
    });
  }

  it("switches without clearing form values, validation, or the profile session", async () => {
    await startEnglish();
    await $("button=Add product").click();
    await $("#phrase-0").setValue("Lava-louças LG VC2");
    await $("button=Add alternative name").click();
    await $("#phrase-1").setValue("LG dishwasher");
    await $("#max-price").setValue("0");
    await $("button=Add product").click();
    await $("#price-error").waitForDisplayed();

    await chooseLanguage("Português (Brasil)");

    await $("h1=Adicionar produto").waitForDisplayed();
    assert.equal(await $("#phrase-0").getValue(), "Lava-louças LG VC2");
    assert.equal(await $("#phrase-1").getValue(), "LG dishwasher");
    assert.equal(await $("#max-price").getValue(), "0");
    assert.match(await $("#price-error").getText(), /Digite um preço válido/);
    assert.equal(
      await $("button[aria-label='Português (Brasil)']").getAttribute("aria-pressed"),
      "true",
    );
    assert.equal(await $("button[aria-label='English']").getAttribute("aria-pressed"), "false");
    await $("button=Sair").waitForDisplayed();
    await $("strong=Pedro").waitForDisplayed();

    await $(".app-profile").click();
    await $("#max-price").setValue("3.000,00");
    await $("button=Adicionar produto").click();

    await $("strong=Lava-louças LG VC2").waitForDisplayed();
    const product = await $("a*=Lava-louças LG VC2").getText();
    assert.match(product, /2 nomes/);
    assert.match(product.replace(/\s/g, ""), /AtéR\$3\.000,00/);

    await chooseLanguage("English");

    await $("h1=Products").waitForDisplayed();
    const englishProduct = await $("a*=Lava-louças LG VC2").getText();
    assert.match(englishProduct, /2 names/);
    assert.match(englishProduct.replace(/\s/g, ""), /UptoR\$3\.000,00/);
  });

  it("preserves a manual choice across reload and sign-out and translates native errors", async () => {
    await command("configure", { scenario: { authorized: true, signOutError: "offline" } });
    await startEnglish();
    await chooseLanguage("Português (Brasil)");
    await $("button=Sair").click();
    await $("[role=alert]").waitForDisplayed();

    assert.match(await $("[role=alert]").getText(), /Não foi possível acessar o Telegram/);

    await $("button[aria-label='English']").click();

    assert.match(await $("[role=alert]").getText(), /Could not reach Telegram/);

    await $("button[aria-label='Português (Brasil)']").click();
    await browser.refresh();
    await enableApp();

    await $("h1=Produtos").waitForDisplayed();
    assert.equal(await browser.execute(() => document.documentElement.lang), "pt-BR");

    await command("configure", { scenario: { authorized: true } });
    await $(".app-profile").click();
    await $("button=Sair").click();

    await $("h1=Conectar ao Telegram").waitForDisplayed();
    assert.equal(await browser.execute(() => localStorage.getItem("skopos.language")), "pt-BR");

    await browser.refresh();
    await enableApp();

    await $("h1=Conectar ao Telegram").waitForDisplayed();
    await $("button=Número de telefone").click();
    await $("#phone").setValue("+5511999999999");
    await $("button=Enviar código de login").click();
    await $("#verification-code").waitForDisplayed();
    assert.match(await $(".form-help").getText(), /código enviado ao seu aplicativo do Telegram/);
  });

  it("loads the bundled Geist font without relying on an installed system font", async () => {
    await startEnglish();

    const typography = await browser.execute(async () => {
      const fonts = await document.fonts.load('400 14px "Geist"', "Produtos · R$ 3.000,00 · çãé");
      await document.fonts.ready;
      const response = await fetch("/fonts/geist/OFL.txt");

      return {
        count: fonts.length,
        statuses: fonts.map((font) => font.status),
        family: getComputedStyle(document.body).fontFamily,
        notice: await response.text(),
      };
    });

    assert.ok(typography.count > 0);
    assert.ok(typography.statuses.every((status) => status === "loaded"));
    assert.match(typography.family, /^Geist/);
    assert.match(typography.notice, /SIL OPEN FONT LICENSE/);
  });
});

async function command(name: string, args?: Record<string, unknown>) {
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
  );
}

async function enableApp() {
  await browser.execute(() => {
    (window as Window & { __SKOPOS_E2E_READY__?: boolean }).__SKOPOS_E2E_READY__ = true;
    window.dispatchEvent(new Event("skopos:e2e-ready"));
  });
}

async function startEnglish() {
  await browser.execute(() => {
    localStorage.setItem("skopos.language", "en");
  });
  await enableApp();
  await $("h1=Products").waitForDisplayed();
}

async function chooseLanguage(name: string) {
  await $(".app-profile").click();
  await $(`button[aria-label='${name}']`).click();
}
