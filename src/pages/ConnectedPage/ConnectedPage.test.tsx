import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ConnectedPage } from "./ConnectedPage";

const auth = vi.hoisted(() => ({ status: vi.fn(), signOut: vi.fn() }));
vi.mock("../../telegram", () => ({
  telegram: auth,
  errorMessage: (error: unknown) => String(error),
}));
beforeEach(() => {
  auth.status.mockReset();
  auth.signOut.mockReset();
  auth.signOut.mockResolvedValue(undefined);
});
afterEach(cleanup);

describe("ConnectedPage", () => {
  it("restores a connected session and signs out", async () => {
    auth.status.mockResolvedValue({ authorized: true, displayName: "Pedro" });
    render(
      <MemoryRouter initialEntries={["/connected"]}>
        <Routes>
          <Route path="/connected" element={<ConnectedPage />} />
          <Route path="/login" element={<h1>Authorize Telegram</h1>} />
        </Routes>
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText("Signed in as Pedro")).toBeInTheDocument());
    auth.status.mockResolvedValue({ authorized: false, displayName: null });
    fireEvent.click(screen.getByRole("button", { name: "Disconnect Telegram" }));
    await waitFor(() => {
      expect(auth.signOut).toHaveBeenCalled();
    });
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "Authorize Telegram" })).toBeInTheDocument(),
    );
  });

  it("returns to login when the saved session is no longer authorized", async () => {
    auth.status.mockResolvedValue({ authorized: false, displayName: null });
    render(
      <MemoryRouter initialEntries={["/connected"]}>
        <Routes>
          <Route path="/connected" element={<ConnectedPage />} />
          <Route path="/login" element={<h1>Authorize Telegram</h1>} />
        </Routes>
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText("Authorize Telegram")).toBeInTheDocument());
  });

  it("shows a session lookup failure", async () => {
    auth.status.mockRejectedValue(new Error("Offline"));
    render(
      <MemoryRouter>
        <ConnectedPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Offline"));
    expect(screen.getByRole("button", { name: "Disconnect Telegram" })).toBeDisabled();
  });

  it("shows a sign-out failure and allows retry", async () => {
    auth.status.mockResolvedValue({ authorized: true, displayName: null });
    auth.signOut.mockRejectedValueOnce(new Error("Network unavailable"));
    render(
      <MemoryRouter>
        <ConnectedPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText("Signed in as Telegram user")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "Disconnect Telegram" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Network unavailable"));
    expect(screen.getByRole("button", { name: "Disconnect Telegram" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Disconnect Telegram" }));
    await waitFor(() => {
      expect(auth.signOut).toHaveBeenCalledTimes(2);
    });
  });
});
