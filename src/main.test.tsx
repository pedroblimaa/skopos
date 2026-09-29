import { beforeEach, expect, it, vi } from "vitest";

const root = vi.hoisted(() => ({ render: vi.fn(), createRoot: vi.fn() }));
vi.mock("react-dom/client", () => ({ default: { createRoot: root.createRoot } }));
vi.mock("./App", () => ({ default: () => null }));
vi.mock("@wdio/tauri-plugin", () => ({}));

beforeEach(() => {
  vi.resetModules();
  root.render.mockReset();
  root.createRoot.mockReset().mockReturnValue({ render: root.render });
  document.body.innerHTML = '<div id="root"></div>';
});

it("mounts the app in the desktop root", async () => {
  await import("./main");
  await vi.waitFor(() => {
    expect(root.render).toHaveBeenCalledTimes(1);
  });
  expect(root.createRoot).toHaveBeenCalledWith(document.getElementById("root"));
});

it("waits for test mocks before mounting an E2E build", async () => {
  vi.stubEnv("VITE_E2E", "1");
  const listen = vi.spyOn(window, "addEventListener");
  try {
    await import("./main");
    expect(root.render).not.toHaveBeenCalled();
    await vi.waitFor(() => {
      expect(listen).toHaveBeenCalledWith("skopos:e2e-ready", expect.any(Function), { once: true });
    });
    window.dispatchEvent(new Event("skopos:e2e-ready"));
    await vi.waitFor(() => {
      expect(root.render).toHaveBeenCalledTimes(1);
    });
  } finally {
    vi.unstubAllEnvs();
    listen.mockRestore();
  }
});
