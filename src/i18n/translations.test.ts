import { describe, expect, it } from "vitest";
import { staticMessageCodes } from "../app-message";
import { translate } from "./translations";

describe("typed translations", () => {
  it.each(staticMessageCodes)("provides both languages for %s", (code) => {
    expect(translate("en", code)).toBeTruthy();
    expect(translate("pt-BR", code)).toBeTruthy();
    expect(translate("pt-BR", code)).not.toBe(translate("en", code));
  });

  it("preserves user data and BRL formatting in interpolated strings", () => {
    expect(translate("en", "upToPrice", { price: "R$ 3.000,00" })).toBe("Up to R$ 3.000,00");
    expect(translate("pt-BR", "upToPrice", { price: "R$ 3.000,00" })).toBe("Até R$ 3.000,00");
    expect(translate("pt-BR", "passwordHint", { hint: "My <hint>" })).toBe("Dica: My <hint>");
    expect(translate("en", "multipleNames", { count: 2 })).toBe("2 names");
    expect(translate("en", "multipleNames")).toBe("{count} names");
  });
});
