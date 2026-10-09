import "@testing-library/jest-dom/vitest";
import type { ReactElement } from "react";
import { render as renderReact } from "@testing-library/react";
import { beforeEach, vi } from "vitest";
import { LanguageProvider } from "./components/LanguageProvider/LanguageProvider";

const computedStyle =
  typeof window === "undefined" ? undefined : window.getComputedStyle.bind(window);

beforeEach(() => {
  if (!computedStyle) return;

  vi.stubGlobal("getComputedStyle", (element: Element, pseudo?: string | null) => {
    const style = computedStyle(element, pseudo);
    style.setProperty("--motion-feedback", "180ms");
    style.setProperty("--motion-page", "220ms");

    return style;
  });

  // JSDOM has no layout engine; observer-driven geometry is covered in desktop tests.
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent: () => false,
  }));

  if (typeof localStorage === "undefined") return;

  localStorage.clear();
  localStorage.setItem("skopos.language", "en");
});

export function render(ui: ReactElement) {
  return renderReact(ui, { wrapper: LanguageProvider });
}
