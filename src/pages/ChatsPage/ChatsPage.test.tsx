import { render } from "../../test-setup";
import { act, cleanup, fireEvent, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { ChatsPage } from "./ChatsPage";
import { ChatsContext } from "./chats-context";
import { useChatCache } from "./useChatCache";
import type { TelegramChat } from "../../telegram.model";

const api = vi.hoisted(() => ({
  listChats: vi.fn(),
  getSelectedChats: vi.fn(),
  saveSelectedChats: vi.fn(),
  getChatPhoto: vi.fn(),
}));
vi.mock("../../telegram", () => ({ telegram: api }));
const chats: TelegramChat[] = [
  { id: "chat:9007199254740993", title: "Offers", kind: "group", username: null, available: true },
  { id: "channel:2", title: "Deals", kind: "channel", username: "deals", available: true },
];

function Harness() {
  const cache = useChatCache();
  const [isOpen, setIsOpen] = useState(true);

  return (
    <ChatsContext.Provider value={cache}>
      <button
        onClick={() => {
          setIsOpen(!isOpen);
        }}
      >
        Navigate
      </button>
      {isOpen && <ChatsPage />}
    </ChatsContext.Provider>
  );
}

beforeEach(() => {
  vi.resetAllMocks();
  api.listChats.mockResolvedValue(chats);
  api.getSelectedChats.mockResolvedValue([]);
  api.getChatPhoto.mockResolvedValue(null);
  api.saveSelectedChats.mockResolvedValue(undefined);
});
afterEach(cleanup);

describe("chat selection", () => {
  it("prevents overlapping cache writes and accepts another selection after completion", async () => {
    let finish!: () => void;
    api.saveSelectedChats.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
    );
    const { result } = renderHook(useChatCache);
    let pending!: Promise<void>;

    act(() => {
      pending = result.current.save([chats[0]]);
      void result.current.save([chats[1]]);
    });

    expect(api.saveSelectedChats).toHaveBeenCalledTimes(1);
    expect(result.current.isSaving).toBe(true);

    await act(async () => {
      finish();
      await pending;
    });

    expect(result.current.saved).toEqual([chats[0]]);
    expect(result.current.isSaving).toBe(false);

    await act(async () => {
      await result.current.save([chats[1]]);
    });

    expect(api.saveSelectedChats).toHaveBeenCalledTimes(2);
    expect(result.current.saved).toEqual([chats[1]]);
  });

  it("shows photos when available and keeps initials after an optional photo failure", async () => {
    api.getChatPhoto.mockImplementation((id: string) =>
      id === chats[0].id
        ? Promise.resolve("data:image/jpeg;base64,/9g=")
        : Promise.reject(new Error("No photo")),
    );
    const view = render(<Harness />);
    await screen.findByText("Offers");
    await waitFor(() =>
      expect(view.container.querySelector(".chats-avatar img")).toHaveAttribute(
        "src",
        "data:image/jpeg;base64,/9g=",
      ),
    );

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(api.getChatPhoto).toHaveBeenCalledTimes(2);
  });

  it("disables controls while saving and prevents another save", async () => {
    let finish!: () => void;
    api.saveSelectedChats.mockReturnValue(
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
    );
    render(<Harness />);
    fireEvent.click(await screen.findByRole("checkbox", { name: /Offers/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));

    expect(screen.getByRole("button", { name: "Saving…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /Offers/ })).toBeDisabled();
    await act(async () => {
      finish();
      await Promise.resolve();
    });

    expect(api.saveSelectedChats).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Chat selection saved.")).toBeInTheDocument();
  });

  it("keeps a pending save busy across navigation before accepting a newer selection", async () => {
    let finish!: () => void;
    api.saveSelectedChats.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
    );
    render(<Harness />);
    fireEvent.click(await screen.findByRole("checkbox", { name: /Offers/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));

    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));
    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));

    expect(screen.getByRole("button", { name: "Saving…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /Deals/ })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Saving…" }));
    expect(api.saveSelectedChats).toHaveBeenCalledTimes(1);

    await act(async () => {
      finish();
      await Promise.resolve();
    });

    expect(screen.getByRole("checkbox", { name: /Offers/ })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: /Deals/ })).toBeEnabled();

    fireEvent.click(screen.getByRole("checkbox", { name: /Offers/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Deals/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));
    await screen.findByText("Chat selection saved.");

    expect(api.saveSelectedChats).toHaveBeenCalledTimes(2);
    expect(api.saveSelectedChats).toHaveBeenLastCalledWith([chats[1]]);
    expect(screen.getByRole("checkbox", { name: /Offers/ })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: /Deals/ })).toBeChecked();
  });

  it("releases the shared saving state after a save fails while away", async () => {
    let fail!: (reason: unknown) => void;
    api.saveSelectedChats.mockReturnValueOnce(
      new Promise<void>((_, reject) => {
        fail = reject;
      }),
    );
    render(<Harness />);
    fireEvent.click(await screen.findByRole("checkbox", { name: /Offers/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));
    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));

    await act(async () => {
      fail({ code: "chatStorage" });
      await Promise.resolve();
    });
    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));

    expect(screen.getByRole("checkbox", { name: /Offers/ })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: /Deals/ })).toBeEnabled();

    fireEvent.click(screen.getByRole("checkbox", { name: /Deals/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));
    await screen.findByText("Chat selection saved.");

    expect(api.saveSelectedChats).toHaveBeenLastCalledWith([chats[1]]);
  });

  it("loads, filters without losing selections, resets, and saves opaque IDs", async () => {
    render(<Harness />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading");
    fireEvent.click(await screen.findByRole("checkbox", { name: /Offers/ }));
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "@deals" } });

    expect(screen.queryByRole("checkbox", { name: /Offers/ })).not.toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /Deals/ })).not.toBeChecked();

    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));
    await screen.findByText("Chat selection saved.");

    expect(api.saveSelectedChats).toHaveBeenCalledWith([chats[0]]);
    expect(screen.getByRole("button", { name: "Save selection" })).toBeDisabled();

    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("checkbox", { name: /Offers/ }));
    fireEvent.click(screen.getByRole("button", { name: "Reset changes" }));

    expect(screen.getByRole("checkbox", { name: /Offers/ })).toBeChecked();
  });

  it("restores saved selection, discards drafts on navigation, and reuses the cache", async () => {
    api.getSelectedChats.mockResolvedValue([chats[0]]);
    render(<Harness />);
    const checkbox = await screen.findByRole("checkbox", { name: /Offers/ });

    expect(checkbox).toBeChecked();
    fireEvent.click(checkbox);
    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));
    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));

    expect(screen.getByRole("checkbox", { name: /Offers/ })).toBeChecked();
    expect(api.listChats).toHaveBeenCalledTimes(1);
    expect(api.getSelectedChats).toHaveBeenCalledTimes(1);
  });

  it("preserves the draft through refresh and a failed save, then saves zero chats", async () => {
    api.getSelectedChats.mockResolvedValue([chats[0]]);
    api.saveSelectedChats.mockRejectedValueOnce({ code: "chatStorage" });
    render(<Harness />);
    fireEvent.click(await screen.findByRole("checkbox", { name: /Offers/ }));
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save selection" })).toBeEnabled(),
    );

    expect(screen.getByRole("checkbox", { name: /Offers/ })).not.toBeChecked();
    expect(api.getSelectedChats).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));
    await screen.findByRole("alert");

    expect(screen.getByRole("checkbox", { name: /Offers/ })).not.toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await screen.findByText("Chat selection saved.");

    expect(api.saveSelectedChats).toHaveBeenLastCalledWith([]);
  });

  it("keeps unavailable saved chats visible and allows deselection", async () => {
    api.listChats.mockResolvedValue([chats[1]]);
    api.getSelectedChats.mockResolvedValue([chats[0]]);
    render(<Harness />);
    const checkbox = await screen.findByRole("checkbox", { name: /Offers.*Unavailable/ });

    expect(checkbox).toBeChecked();
    fireEvent.click(checkbox);

    expect(checkbox).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Save selection" }));
    await waitFor(() => expect(screen.queryByText("Offers")).not.toBeInTheDocument());
  });

  it("retries a complete load after failure and shows empty and no-result states", async () => {
    api.listChats.mockRejectedValueOnce({ code: "chatLoadFailed" }).mockResolvedValueOnce([]);
    render(<Harness />);
    await screen.findByRole("alert");

    expect(screen.queryByRole("list")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await screen.findByText("No groups or channels yet");

    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await screen.findByText("Offers");
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "absent" } });

    expect(screen.getByText("No chats match your search.")).toBeInTheDocument();
  });

  it("blocks duplicate requests and ignores late responses after unmount", async () => {
    let finish!: (value: TelegramChat[]) => void;
    api.listChats.mockReturnValue(
      new Promise<TelegramChat[]>((resolve) => {
        finish = resolve;
      }),
    );
    const view = render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));
    fireEvent.click(screen.getByRole("button", { name: "Navigate" }));

    expect(api.listChats).toHaveBeenCalledTimes(1);
    view.unmount();
    await act(async () => {
      finish(chats);
      await Promise.resolve();
    });

    expect(api.getChatPhoto).not.toHaveBeenCalled();
  });
});
