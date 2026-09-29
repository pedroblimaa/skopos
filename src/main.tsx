import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./global.css";

async function start() {
  if (import.meta.env.VITE_E2E) {
    await import("@wdio/tauri-plugin");
    const testWindow = window as Window & { __SKOPOS_E2E_READY__?: boolean };
    if (!testWindow.__SKOPOS_E2E_READY__) {
      await new Promise<void>((resolve) => {
        window.addEventListener(
          "skopos:e2e-ready",
          () => {
            resolve();
          },
          { once: true },
        );
      });
    }
  }

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void start();
