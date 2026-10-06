import { expect, it } from "vitest";
import { isMinimumPriceInvalid, parsePriceCents } from "./price";

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

it("allows zero only for the minimum and retains validation", () => {
  expect(parsePriceCents("0", true)).toBe(0);
  expect(parsePriceCents("R$ 0,00", true)).toBe(0);
  expect(parsePriceCents("12,34", true)).toBe(1234);
  expect(parsePriceCents("", true)).toBeNull();
  expect(parsePriceCents("-1", true)).toBeNaN();
  expect(parsePriceCents("0")).toBeNaN();
});

it("validates minimum and maximum relationships including automatic and uncapped modes", () => {
  expect(isMinimumPriceInvalid(NaN, 500000)).toBe(true);
  expect(isMinimumPriceInvalid(500001, 500000)).toBe(true);
  expect(isMinimumPriceInvalid(500000, 500000)).toBe(false);
  expect(isMinimumPriceInvalid(null, 500000)).toBe(false);
  expect(isMinimumPriceInvalid(1800, null)).toBe(false);
});
