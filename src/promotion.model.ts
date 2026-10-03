import type { AppMessage } from "./app-message";

export interface SourceMessage {
  chatId: string;
  chatTitle: string;
  messageId: number;
  postedAt: number;
  text: string;
  messageLink: string | null;
  image?: string | null;
}

export interface ProductMatch {
  watchId: number;
  priceCents: number | null;
  message: SourceMessage;
}

export interface SearchSummary {
  startedAt: number;
  since: number;
  completedChats: number;
  totalChats: number;
  unavailableChats: string[];
  failure: AppMessage | null;
}

export interface SearchResults {
  matches: ProductMatch[];
  summary: SearchSummary | null;
}
