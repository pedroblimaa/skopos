import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { AppShell } from "./AppShell";
import type { SessionStatus } from "../../telegram";

const api = vi.hoisted(() => ({ status: vi.fn(), signOut: vi.fn() }));
vi.mock("../../telegram", () => ({
  telegram: api,
  errorMessage: (error: unknown) => String(error),
}));
beforeEach(() => {
  vi.resetAllMocks();
  api.status.mockResolvedValue({ authorized: true, displayName: "Pedro" });
  api.signOut.mockResolvedValue(undefined);
});
afterEach(cleanup);

function renderShell() {
  return render(
    <MemoryRouter initialEntries={["/connected"]}>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/connected" element={<h1>Products</h1>} />
        </Route>
        <Route path="/login" element={<h1>Authorize Telegram</h1>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("AppShell", () => {
  it("loads the profile before showing protected content", async () => {
    renderShell();

    expect(screen.getByRole("status")).toHaveTextContent("Loading your Telegram profile");
    expect(screen.getByRole("button", { name: "Telegram profile" })).toBeDisabled();
    expect(screen.queryByRole("heading", { name: "Products" })).not.toBeInTheDocument();
    expect(await screen.findByText("Pedro")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Products" })).toBeInTheDocument();

    const trigger = screen.getByRole("button", { name: "Telegram profile" });
    expect(trigger).toHaveAttribute("popovertarget", "telegram-profile-menu");

    const menu = document.getElementById("telegram-profile-menu");
    if (!menu) throw new Error("Profile menu is missing");
    fireEvent(menu, Object.assign(new Event("toggle"), { newState: "open" }));

    expect(trigger).toHaveAttribute("aria-expanded", "true");
    fireEvent(menu, Object.assign(new Event("toggle"), { newState: "closed" }));

    expect(trigger).toHaveAttribute("aria-expanded", "false");
  });

  it("redirects unauthorized sessions", async () => {
    api.status.mockResolvedValue({ authorized: false, displayName: null });

    renderShell();

    expect(await screen.findByRole("heading", { name: "Authorize Telegram" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Products" })).not.toBeInTheDocument();
  });

  it("reports profile errors without exposing protected content", async () => {
    api.status.mockRejectedValue(new Error("Offline"));

    renderShell();

    expect(await screen.findByRole("alert")).toHaveTextContent("Offline");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Products" })).not.toBeInTheDocument();
  });

  it("keeps the session after sign-out failure and allows retry", async () => {
    api.status.mockResolvedValue({ authorized: true, displayName: null });
    api.signOut.mockRejectedValueOnce(new Error("Network unavailable"));

    renderShell();
    await screen.findByText("Telegram user");

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

    expect(screen.getByRole("button", { name: "Signing out…" })).toBeDisabled();
    expect(await screen.findByRole("alert")).toHaveTextContent("Network unavailable");
    expect(screen.getByRole("heading", { name: "Products" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    await screen.findByRole("heading", { name: "Authorize Telegram" });
    expect(api.signOut).toHaveBeenCalledTimes(2);
  });

  it.each([true, false])("ignores a profile %s result after unmount", async (succeeds) => {
    let resolve!: (value: SessionStatus) => void;
    let reject!: (reason: Error) => void;
    api.status.mockReturnValue(
      new Promise<SessionStatus>((res, rej) => {
        resolve = res;
        reject = rej;
      }),
    );
    const view = renderShell();
    view.unmount();
    await act(async () => {
      if (succeeds) resolve({ authorized: false, displayName: null });
      else reject(new Error("Late failure"));
      await Promise.resolve();
    });
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });
});
