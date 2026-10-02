import { useEffect, useState, type ReactNode } from "react";
import { LanguageContext, type LanguageContextValue } from "../../i18n/language-context";
import { translate, type Language } from "../../i18n/translations";

const storageKey = "skopos.language";

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [language, updateLanguage] = useState(initialLanguage);

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  function setLanguage(next: Language) {
    updateLanguage(next);

    try {
      localStorage.setItem(storageKey, next);
    } catch {
      // Switching remains available when the WebView denies preference storage.
    }
  }

  const value: LanguageContextValue = {
    language,
    setLanguage,
    t: (key, params) => translate(language, key, params),
    message: (message) =>
      translate(language, message.code, "params" in message ? message.params : undefined),
  };

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

function initialLanguage(): Language {
  try {
    const saved = localStorage.getItem(storageKey);
    if (saved === "pt-BR" || saved === "en") return saved;
  } catch {
    // Use the system language when no saved preference can be read.
  }

  const systemLanguage = navigator.language || navigator.languages[0] || "en";

  return /^pt(?:-|$)/i.test(systemLanguage) ? "pt-BR" : "en";
}
