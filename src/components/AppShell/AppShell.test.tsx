import { render } from "../../test-setup";
import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { AppShell } from "./AppShell";
import type { SessionStatus } from "../../telegram";

const api = vi.hoisted(() => ({
  status: vi.fn(),
  signOut: vi.fn(),
  getProfilePhoto: vi.fn(),
  onNotificationStatus: vi.fn(),
}));
vi.mock("../../telegram", () => ({
  telegram: api,
}));
beforeEach(() => {
  vi.resetAllMocks();
  api.status.mockResolvedValue({ authorized: true, displayName: "Pedro" });
  api.signOut.mockResolvedValue(undefined);
  api.getProfilePhoto.mockResolvedValue(null);
  api.onNotificationStatus.mockResolvedValue(vi.fn());
});
afterEach(cleanup);

function renderShell() {
  return render(
    <MemoryRouter initialEntries={["/connected"]}>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/connected" element={<h1>Products</h1>} />
          <Route path="/chats" element={<h1>Chats</h1>} />
        </Route>
        <Route path="/login" element={<h1>Authorize Telegram</h1>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("AppShell", () => {
  it("shows content before the photo arrives and keeps the avatar across navigation", async () => {
    let resolvePhoto!: (photo: string) => void;
    api.getProfilePhoto.mockReturnValue(
      new Promise<string>((resolve) => {
        resolvePhoto = resolve;
      }),
    );
    renderShell();
    await screen.findByRole("heading", { name: "Products" });

    expect(document.querySelector(".app-profile-icon svg")).toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();

    await act(async () => {
      resolvePhoto("data:image/jpeg;base64,/9g=");
      await Promise.resolve();
    });
    const avatar = document.querySelector(".app-profile-icon img");

    expect(avatar).toHaveAttribute("src", "data:image/jpeg;base64,/9g=");
    expect(avatar).toHaveAttribute("alt", "");

    fireEvent.click(screen.getByRole("link", { name: "Chats" }));
    await screen.findByRole("heading", { name: "Chats" });
    fireEvent.click(screen.getByRole("link", { name: "Products" }));
    await screen.findByRole("heading", { name: "Products" });

    expect(document.querySelector(".app-profile-icon img")).toBe(avatar);
    expect(api.getProfilePhoto).toHaveBeenCalledTimes(1);
  });

  it.each(["missing", "download", "image"])(
    "keeps the menu usable with a %s photo failure",
    async (failure) => {
      if (failure === "download") api.getProfilePhoto.mockRejectedValue({ code: "authNetwork" });
      if (failure === "image") api.getProfilePhoto.mockResolvedValue("data:image/jpeg;base64,/9g=");
      renderShell();
      await screen.findByText("Pedro");

      if (failure === "image") {
        await waitFor(() =>
          expect(document.querySelector(".app-profile-icon img")).toBeInTheDocument(),
        );
        const image = document.querySelector(".app-profile-icon img");
        if (!image) throw new Error("Profile image is missing");

        fireEvent.error(image);
      }

      await waitFor(() =>
        expect(document.querySelector(".app-profile-icon svg")).toBeInTheDocument(),
      );
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Telegram profile" })).toBeEnabled();
      expect(screen.getByRole("heading", { name: "Products" })).toBeInTheDocument();
    },
  );

  it.each([true, false])(
    "ignores a late photo result after unmount (success: %s)",
    async (succeeds) => {
      let resolvePhoto!: (photo: string) => void;
      let rejectPhoto!: (reason: Error) => void;
      api.getProfilePhoto.mockReturnValue(
        new Promise<string>((resolve, reject) => {
          resolvePhoto = resolve;
          rejectPhoto = reject;
        }),
      );
      const view = renderShell();
      await screen.findByText("Pedro");
      view.unmount();

      await act(async () => {
        if (succeeds) resolvePhoto("data:image/jpeg;base64,/9g=");
        else rejectPhoto(new Error("Late photo failure"));
        await Promise.resolve();
      });

      expect(document.querySelector(".app-profile-icon")).not.toBeInTheDocument();
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    },
  );

  it("switches the language above sign-out and retranslates an existing error", async () => {
    api.signOut.mockRejectedValueOnce({ code: "authNetwork" });
    renderShell();
    await screen.findByText("Pedro");
    const portuguese = screen.getByRole("button", { name: "Português (Brasil)" });
    const english = screen.getByRole("button", { name: "English" });

    expect(english).toHaveAttribute("aria-pressed", "true");
    expect(portuguese).toHaveAttribute("aria-pressed", "false");
    expect(
      portuguese.compareDocumentPosition(screen.getByRole("button", { name: "Sign out" })) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not reach Telegram");

    fireEvent.click(portuguese);

    expect(screen.getByRole("group", { name: "Idioma" })).toBeInTheDocument();
    expect(portuguese).toHaveAttribute("aria-pressed", "true");
    expect(english).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByRole("button", { name: "Sair" })).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("Não foi possível acessar o Telegram");
    expect(api.status).toHaveBeenCalledTimes(1);
    expect(api.signOut).toHaveBeenCalledTimes(1);

    fireEvent.click(english);

    expect(screen.getByRole("group", { name: "Language" })).toBeInTheDocument();
    expect(localStorage.getItem("skopos.language")).toBe("en");
  });

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
    expect(api.getProfilePhoto).not.toHaveBeenCalled();
  });

  it("reports profile errors without exposing protected content", async () => {
    api.status.mockRejectedValue({ code: "authNetwork" });

    renderShell();

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not reach Telegram");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Products" })).not.toBeInTheDocument();
  });

  it("keeps the session after sign-out failure and allows retry", async () => {
    api.status.mockResolvedValue({ authorized: true, displayName: null });
    api.signOut.mockRejectedValueOnce({ code: "authNetwork" });

    renderShell();
    await screen.findByText("Telegram user");

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

    expect(screen.getByRole("button", { name: "Signing out…" })).toBeDisabled();
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not reach Telegram");
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
