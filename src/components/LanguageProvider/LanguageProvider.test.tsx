import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LanguageProvider } from "./LanguageProvider";
import { useLanguage } from "../../i18n/useLanguage";

function Preview() {
  const { language, setLanguage, t, message } = useLanguage();

  return (
    <>
      <output>{language}</output>
      <h1>{t("products")}</h1>
      <p>{t("multipleNames", { count: 2 })}</p>
      <p>{message({ code: "authNetwork" })}</p>
      <p>{message({ code: "floodWaitSeconds", params: { seconds: 30 } })}</p>
      <input aria-label="draft" defaultValue="Lava-louças LG · R$ 3.000,00" />
      <button
        onClick={() => {
          setLanguage("pt-BR");
        }}
      >
        Português
      </button>
      <button
        onClick={() => {
          setLanguage("en");
        }}
      >
        English
      </button>
    </>
  );
}

beforeEach(() => {
  localStorage.clear();
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("LanguageProvider", () => {
  it.each(["pt-BR", "pt-PT", "pt", "PT-ao"])("defaults %s to Brazilian Portuguese", (locale) => {
    vi.spyOn(navigator, "language", "get").mockReturnValue(locale);

    render(
      <LanguageProvider>
        <Preview />
      </LanguageProvider>,
    );

    expect(screen.getByRole("heading")).toHaveTextContent("Produtos");
    expect(document.documentElement.lang).toBe("pt-BR");
    expect(localStorage.getItem("skopos.language")).toBeNull();
  });

  it.each(["en-US", "es-ES", "fr-FR"])("defaults %s to English", (locale) => {
    vi.spyOn(navigator, "language", "get").mockReturnValue(locale);

    render(
      <LanguageProvider>
        <Preview />
      </LanguageProvider>,
    );

    expect(screen.getByRole("heading")).toHaveTextContent("Products");
  });

  it.each(["en", "pt-BR"])(
    "restores the manual %s preference ahead of the system language",
    (language) => {
      localStorage.setItem("skopos.language", language);
      vi.spyOn(navigator, "language", "get").mockReturnValue(language === "en" ? "pt-BR" : "en-US");

      render(
        <LanguageProvider>
          <Preview />
        </LanguageProvider>,
      );

      expect(screen.getByRole("status")).toHaveTextContent(language);
    },
  );

  it("updates text, parameters and document language without replacing a draft", () => {
    localStorage.setItem("skopos.language", "en");
    const view = render(
      <LanguageProvider>
        <Preview />
      </LanguageProvider>,
    );
    const input = screen.getByLabelText("draft");
    fireEvent.change(input, { target: { value: "RTX 5070" } });

    fireEvent.click(screen.getByRole("button", { name: "Português" }));

    expect(screen.getByRole("heading")).toHaveTextContent("Produtos");
    expect(screen.getByText("2 nomes")).toBeInTheDocument();
    expect(screen.getByText(/Não foi possível acessar o Telegram/)).toBeInTheDocument();
    expect(screen.getByText(/30 segundos/)).toBeInTheDocument();
    expect(screen.getByLabelText("draft")).toBe(input);
    expect(input).toHaveValue("RTX 5070");
    expect(document.documentElement.lang).toBe("pt-BR");
    expect(localStorage.getItem("skopos.language")).toBe("pt-BR");

    view.unmount();
    render(
      <LanguageProvider>
        <Preview />
      </LanguageProvider>,
    );

    expect(screen.getByRole("heading")).toHaveTextContent("Produtos");

    fireEvent.click(screen.getByRole("button", { name: "English" }));

    expect(screen.getByRole("heading")).toHaveTextContent("Products");
    expect(document.documentElement.lang).toBe("en");
    expect(localStorage.getItem("skopos.language")).toBe("en");
  });

  it("ignores an unsupported saved value and uses the language list when necessary", () => {
    localStorage.setItem("skopos.language", "fr");
    vi.spyOn(navigator, "language", "get").mockReturnValue("");
    vi.spyOn(navigator, "languages", "get").mockReturnValue(["pt-PT"]);

    render(
      <LanguageProvider>
        <Preview />
      </LanguageProvider>,
    );

    expect(screen.getByRole("heading")).toHaveTextContent("Produtos");
  });

  it("defaults to English when the system provides no language", () => {
    vi.spyOn(navigator, "language", "get").mockReturnValue("");
    vi.spyOn(navigator, "languages", "get").mockReturnValue([]);

    render(
      <LanguageProvider>
        <Preview />
      </LanguageProvider>,
    );

    expect(screen.getByRole("heading")).toHaveTextContent("Products");
  });

  it("supports switching when preference storage is unavailable", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("Denied");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("Denied");
    });
    vi.spyOn(navigator, "language", "get").mockReturnValue("en-US");

    render(
      <LanguageProvider>
        <Preview />
      </LanguageProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Português" }));

    expect(screen.getByRole("heading")).toHaveTextContent("Produtos");
  });

  it("requires a provider for language consumers", () => {
    expect(() => render(<Preview />)).toThrow("useLanguage requires a LanguageProvider");
  });
});
