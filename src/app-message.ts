export const staticMessageCodes = [
  "unexpectedError",
  "authStorage",
  "authCancelled",
  "authNetwork",
  "authLoginFailed",
  "floodWait",
  "invalidPhone",
  "codeExpired",
  "invalidCode",
  "incorrectPassword",
  "emailSetupRequired",
  "additionalLoginStep",
  "missingCode",
  "signUpRequired",
  "passwordVerificationFailed",
  "loginIncomplete",
  "internationalPhoneRequired",
  "signOutInProgress",
  "requestCodeFirst",
  "restartLogin",
  "missingCredentials",
  "invalidApiId",
  "missingDataCenter",
  "qrMigrationFailed",
  "qrLoginIncomplete",
  "invalidPhrase",
  "invalidPrice",
  "productNotFound",
  "watchStorage",
  "watchStorageOpen",
  "deliveryApp",
  "deliverySms",
  "deliveryCall",
  "deliveryFlashCall",
  "deliverySmsPhrase",
] as const;

export type AppMessage =
  | { code: (typeof staticMessageCodes)[number] }
  | { code: "floodWaitSeconds"; params: { seconds: number } }
  | { code: "telegramRejected"; params: { name: string } }
  | { code: "deliveryEmail"; params: { email: string } }
  | { code: "deliveryFragment"; params: { url: string } }
  | { code: "deliveryMissedCall"; params: { prefix: string } };

export function appError(reason: unknown): AppMessage {
  if (isAppMessage(reason)) return reason;

  if (typeof reason === "object" && reason !== null && "message" in reason) {
    if (isAppMessage(reason.message)) return reason.message;
  }

  return { code: "unexpectedError" };
}

export function isAppMessage(value: unknown): value is AppMessage {
  if (typeof value !== "object" || value === null || !("code" in value)) return false;

  if (staticMessageCodes.some((code) => code === value.code)) return true;
  if (!("params" in value) || typeof value.params !== "object" || value.params === null) {
    return false;
  }

  const params = value.params;

  switch (value.code) {
    case "floodWaitSeconds":
      return (
        "seconds" in params && typeof params.seconds === "number" && Number.isFinite(params.seconds)
      );
    case "telegramRejected":
      return "name" in params && typeof params.name === "string";
    case "deliveryEmail":
      return "email" in params && typeof params.email === "string";
    case "deliveryFragment":
      return "url" in params && typeof params.url === "string";
    case "deliveryMissedCall":
      return "prefix" in params && typeof params.prefix === "string";
    default:
      return false;
  }
}
