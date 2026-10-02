import { createContext } from "react";
import type { AppMessage } from "../app-message";
import type { Language, TranslationKey } from "./translations";

export interface LanguageContextValue {
  language: Language;
  setLanguage: (language: Language) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
  message: (value: AppMessage) => string;
}

export const LanguageContext = createContext<LanguageContextValue | null>(null);
