export interface SessionStatus {
  authorized: boolean;
  displayName: string | null;
}

export interface QrToken {
  url: string;
  expiresAt: number;
}

export type LoginResult =
  { step: "authorized"; status: SessionStatus } | { step: "passwordRequired"; hint: string | null };

export type CodeRequest =
  | { step: "codeSent"; message: string; length: number | null }
  | { step: "authorized"; status: SessionStatus };
