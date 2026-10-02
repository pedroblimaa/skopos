import type { AppMessage } from "./app-message";

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

export interface CodeSubmissionError {
  message: AppMessage;
  canRetryCode: boolean;
}

export type CodeRequest =
  | { step: "codeSent"; message: AppMessage; length: number | null }
  | { step: "authorized"; status: SessionStatus };
