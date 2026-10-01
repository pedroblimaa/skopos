import assert from "node:assert/strict";
import { $, browser } from "@wdio/globals";
import { beforeEach, describe, it } from "mocha";

interface Watch {
  id: number;
  phrases: string[];
  maxPriceCents: number | null;
}

describe("product desktop flows", () => {
  beforeEach(async () => {
    await browser.refresh();
    await command("configure", { scenario: { authorized: true } });
    const products = await command<Watch[]>("list_watches");
    for (const product of products) await command("delete_watch", { id: product.id });

    await browser.execute(() => {
      window.history.replaceState({}, "", "/connected");
    });
    await enableApp();
    await $("h1=Products").waitForDisplayed();
  });

  it("creates and reloads a locally saved product", async () => {
    await $("button=Add product").click();
    await $("#phrase-0").setValue("Laptop Vivobook S14");
    await $("button=Add alternative name").click();
    await $("#phrase-1").setValue("Asus Vivobook 14");
    await $("#max-price").setValue("3.500,00");
    await $("button=Add product").click();

    await $("strong=Laptop Vivobook S14").waitForDisplayed();
    await reloadProducts();
    await $("strong=Laptop Vivobook S14").waitForDisplayed();
    const products = await command<Watch[]>("list_watches");
    assert.deepEqual(products[0].phrases, ["Laptop Vivobook S14", "Asus Vivobook 14"]);
    assert.equal(products[0].maxPriceCents, 350000);
  });

  it("opens a product for editing and persists changes to the same product", async () => {
    const saved = await seedProduct();

    await $("strong=Laptop Vivobook S14").click();
    await $("h1=Edit product").waitForDisplayed();
    await browser.waitUntil(
      async () => (await $("#phrase-0").getValue()) === "Laptop Vivobook S14",
    );
    assert.equal(await $("#phrase-1").getValue(), "Asus Vivobook 14");
    assert.equal(await $("#max-price").getValue(), "3500,00");

    await $("#phrase-0").setValue("Vivobook OLED");
    await $("button[aria-label='Remove alternative name 1']").click();
    await $("button=Add alternative name").click();
    await $("#phrase-2").setValue("Asus OLED S14");
    await $("#max-price").clearValue();
    await $("button=Save changes").click();
    await $("strong=Vivobook OLED").waitForDisplayed();

    await reloadProducts();
    await $("strong=Vivobook OLED").waitForDisplayed();
    const products = await command<Watch[]>("list_watches");
    assert.deepEqual(products, [
      { id: saved.id, phrases: ["Vivobook OLED", "Asus OLED S14"], maxPriceCents: null },
    ]);
  });

  it("can cancel deletion, then delete only the selected product permanently", async () => {
    const saved = await seedProduct();
    const other = await command<Watch>("create_watch", {
      input: { phrases: ["RTX 5070"], maxPriceCents: null },
    });
    await reloadProducts();
    await $("strong=RTX 5070").waitForDisplayed();

    await $("button[aria-label='Delete Laptop Vivobook S14']").click();
    await $("button=Keep product").click();
    assert.equal((await command<Watch[]>("list_watches")).length, 2);

    await $("button[aria-label='Delete Laptop Vivobook S14']").click();
    await $("button=Delete product").click();
    await browser.waitUntil(async () => !(await $("strong=Laptop Vivobook S14").isExisting()));
    await reloadProducts();
    await $("strong=RTX 5070").waitForDisplayed();
    assert.deepEqual(await command<Watch[]>("list_watches"), [other]);
    await assert.rejects(
      command("update_watch", {
        id: saved.id,
        input: { phrases: ["Deleted"], maxPriceCents: null },
      }),
      /no longer exists/,
    );
  });

  it("shows validation and a missing-product save error without losing entered values", async () => {
    const saved = await seedProduct();
    await $("strong=Laptop Vivobook S14").click();
    await $("h1=Edit product").waitForDisplayed();
    await $("#phrase-0").waitForDisplayed();
    await $("#phrase-0").clearValue();
    await $("#max-price").setValue("0");
    await $("button=Save changes").click();

    assert.equal(await $("#phrase-0").getAttribute("aria-invalid"), "true");
    assert.equal(await $("#max-price").getAttribute("aria-invalid"), "true");
    assert.equal(await $("button=Save changes").isEnabled(), false);

    await $("#phrase-0").setValue("Edited laptop");
    await $("#max-price").setValue("3.200,00");
    await command("delete_watch", { id: saved.id });
    await $("button=Save changes").click();
    await $("[role=alert]").waitForDisplayed();

    assert.match(await $("[role=alert]").getText(), /no longer exists/);
    assert.equal(await $("#phrase-0").getValue(), "Edited laptop");
    assert.equal(await $("#max-price").getValue(), "3.200,00");
    await $("nav[aria-label=Breadcrumb] a").click();
    await $("h1=Products").waitForDisplayed();
  });

  it("reveals name matching help on focus and dismisses with Escape", async () => {
    await $("button=Add product").click();
    const trigger = $("button[aria-label='How search names match']");
    const tooltip = $("[role=tooltip]");
    await trigger.waitForDisplayed();
    assert.equal(await tooltip.isDisplayed(), false);

    await trigger.click();
    await tooltip.waitForDisplayed();
    assert.match(await tooltip.getText(), /Every word.*\(AND\).*Any name.*\(OR\)/);
    await browser.keys("Escape");
    await tooltip.waitForDisplayed({ reverse: true });

    await $("#phrase-0").click();
    assert.equal(await $("#phrase-0").isFocused(), true);

    await trigger.click();
    assert.equal(await trigger.isFocused(), true);
    await tooltip.waitForDisplayed();
  });

  it("opens and dismisses the profile menu, then signs out without deleting products", async () => {
    const saved = await seedProduct();
    const trigger = $(".app-profile");
    const menu = $("#telegram-profile-menu");
    await trigger.click();
    await menu.waitForDisplayed();
    assert.equal(await trigger.getAttribute("aria-expanded"), "true");

    await trigger.click();
    await menu.waitForDisplayed({ reverse: true });
    assert.equal(await trigger.getAttribute("aria-expanded"), "false");
    await trigger.click();
    await $("button=Sign out").click();
    await $("h1=Authorize Telegram").waitForDisplayed();
    assert.deepEqual(await command<Watch[]>("list_watches"), [saved]);
  });
});

async function seedProduct() {
  const saved = await command<Watch>("create_watch", {
    input: { phrases: ["Laptop Vivobook S14", "Asus Vivobook 14"], maxPriceCents: 350000 },
  });
  await reloadProducts();
  await $("strong=Laptop Vivobook S14").waitForDisplayed();
  return saved;
}

async function reloadProducts() {
  await browser.refresh();
  await enableApp();
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
