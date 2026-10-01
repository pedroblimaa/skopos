import "@testing-library/jest-dom/vitest";
import type { ReactElement } from "react";
import { render as renderReact } from "@testing-library/react";
import { beforeEach } from "vitest";
import { LanguageProvider } from "./components/LanguageProvider/LanguageProvider";

beforeEach(() => {
  if (typeof localStorage === "undefined") return;

  localStorage.clear();
  localStorage.setItem("skopos.language", "en");
});

export function render(ui: ReactElement) {
  return renderReact(ui, { wrapper: LanguageProvider });
}
