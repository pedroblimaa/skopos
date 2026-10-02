import { describe, expect, it } from "vitest";
import { appError, isAppMessage, staticMessageCodes } from "./app-message";

describe("message boundary", () => {
  it.each(staticMessageCodes)("accepts the %s message code", (code) => {
    expect(appError({ code })).toEqual({ code });
  });

  it.each([
    { code: "floodWaitSeconds", params: { seconds: 30 } },
    { code: "telegramRejected", params: { name: "UNKNOWN_RPC" } },
    { code: "deliveryEmail", params: { email: "p***@example.com" } },
    { code: "deliveryFragment", params: { url: "https://t.me/code" } },
    { code: "deliveryMissedCall", params: { prefix: "+55" } },
  ])("preserves parameters in $code errors and code-submission failures", (message) => {
    expect(appError(message)).toEqual(message);
    expect(appError({ message, canRetryCode: false })).toEqual(message);
  });

  it.each([
    null,
    undefined,
    42,
    "raw failure",
    new Error("private details"),
    {},
    { message: "private details" },
    { code: "unknown" },
    { code: "deliveryEmail" },
    { code: "deliveryEmail", params: null },
    { code: "deliveryEmail", params: "private" },
    { code: "deliveryEmail", params: {} },
    { code: "deliveryEmail", params: { email: 42 } },
    { code: "floodWaitSeconds", params: { seconds: Infinity } },
    { code: "floodWaitSeconds", params: { seconds: "30" } },
    { code: "floodWaitSeconds", params: {} },
    { code: "telegramRejected", params: { name: 42 } },
    { code: "deliveryFragment", params: { url: 42 } },
    { code: "deliveryMissedCall", params: { prefix: 42 } },
    { code: "unknown", params: {} },
  ])("rejects malformed or untranslated payloads: %j", (value) => {
    expect(isAppMessage(value)).toBe(false);
    expect(appError(value)).toEqual({ code: "unexpectedError" });
  });
});
