import { expect, it } from "vitest";
import { parsePriceCents } from "./price";

it("parses optional Brazilian real amounts into integer cents", () => {
  expect(parsePriceCents(" ")).toBeNull();
  expect(parsePriceCents("3.500,00")).toBe(350000);
  expect(parsePriceCents("R$ 4.000")).toBe(400000);
  expect(parsePriceCents("12,5")).toBe(1250);
  expect(parsePriceCents("1.234.567,89")).toBe(123456789);
});

it.each(["0", "-1", "abc", "3.50", "1,234", "999999999999999", "R$ 0,00"])(
  "rejects invalid amount %s",
  (value) => {
    expect(parsePriceCents(value)).toBeNaN();
  },
);
