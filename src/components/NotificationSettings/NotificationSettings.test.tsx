import { render } from "../../test-setup";
import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { useNotificationPreferences } from "./useNotificationPreferences";
import { NotificationSettings } from "./NotificationSettings";
import type { NotificationSettings as Settings } from "../../notification.model";

const api = vi.hoisted(() => ({
  onNotificationStatus: vi.fn(),
  notificationSettings: vi.fn(),
  notificationStatus: vi.fn(),
  saveNotificationSettings: vi.fn(),
  retryUncertainNotifications: vi.fn(),
}));
vi.mock("../../telegram", () => ({ telegram: api }));
const defaults: Settings = {
  telegramEnabled: true,
  desktopEnabled: true,
  language: "en",
};

beforeEach(() => {
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.setAttribute("open", "");
        this.querySelector<HTMLButtonElement>("button")?.focus();
      },
    },
    close: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.removeAttribute("open");
      },
    },
  });
  vi.resetAllMocks();
  api.onNotificationStatus.mockResolvedValue(vi.fn());
  api.notificationSettings.mockResolvedValue(defaults);
  api.notificationStatus.mockResolvedValue({ pending: 0, uncertain: 0, failure: null });
  api.saveNotificationSettings.mockImplementation((settings: Settings) =>
    Promise.resolve(settings),
  );
  api.retryUncertainNotifications.mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
});

describe("notification settings", () => {
  it("shows settings while delivery status is still loading", async () => {
    api.notificationStatus.mockReturnValue(new Promise(() => {}));
    render(<PreferencesDialog onClose={vi.fn()} />);

    expect(await screen.findByRole("switch", { name: "Save to Telegram" })).toBeChecked();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("reuses preloaded records when closing and reopening", async () => {
    render(<PreferencesDialog onClose={vi.fn()} />);
    await screen.findByRole("switch", { name: "Save to Telegram" });

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "Reopen" }));

    expect(screen.getByRole("switch", { name: "Save to Telegram" })).toBeChecked();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(api.notificationSettings).toHaveBeenCalledOnce();
    expect(api.notificationStatus).toHaveBeenCalledOnce();
  });

  it("starts with both switches on, saves each independently, and closes", async () => {
    const onClose = vi.fn();

    render(<PreferencesDialog onClose={onClose} />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading notification settings");

    const telegram = await screen.findByRole("switch", { name: "Save to Telegram" });
    const desktop = screen.getByRole("switch", { name: "Desktop alerts" });

    expect(telegram).toBeChecked();
    expect(desktop).toBeChecked();

    fireEvent.click(telegram);
    await waitFor(() => expect(telegram).not.toBeChecked());

    expect(desktop).toBeChecked();
    expect(api.saveNotificationSettings).toHaveBeenLastCalledWith({
      ...defaults,
      telegramEnabled: false,
    });

    fireEvent.click(desktop);
    await waitFor(() => expect(desktop).not.toBeChecked());
    fireEvent.click(screen.getByRole("button", { name: "Close" }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  it("focuses Close instead of opening the info tooltip when the dialog opens", async () => {
    render(<PreferencesDialog onClose={vi.fn()} />);
    await screen.findByRole("switch", { name: "Save to Telegram" });

    expect(screen.getByRole("button", { name: "Close" })).toHaveFocus();
    expect(screen.getByRole("button", { name: "About notifications" })).not.toHaveFocus();
  });

  it("explains Saved Messages without showing bot setup", async () => {
    render(<PreferencesDialog onClose={vi.fn()} />);
    await screen.findByRole("switch", { name: "Save to Telegram" });

    expect(screen.getByText(/New promotions go to Saved Messages/u)).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(screen.queryByText("Connect bot")).not.toBeInTheDocument();
  });

  it("explains errors and requires confirmation before retrying uncertain sends", async () => {
    api.notificationStatus.mockResolvedValueOnce({ pending: 0, uncertain: 2, failure: null });
    render(<PreferencesDialog onClose={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", { name: "Retry uncertain messages" }));

    expect(api.retryUncertainNotifications).not.toHaveBeenCalled();
    expect(screen.getByText(/Retrying can send duplicates/u)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Retry anyway" }));
    await waitFor(() => expect(screen.queryByText(/Delivery could not/u)).not.toBeInTheDocument());

    expect(api.retryUncertainNotifications).toHaveBeenCalledOnce();

    api.saveNotificationSettings.mockRejectedValue({ code: "notificationStorage" });

    fireEvent.click(screen.getByRole("switch", { name: "Save to Telegram" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("delivery history");
  });

  it("reports settings load failures", async () => {
    api.notificationSettings.mockRejectedValue({ code: "notificationStorage" });
    render(<PreferencesDialog onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("delivery history");
  });

  it("ignores late settings and command completions after unmount", async () => {
    let resolve!: (settings: Settings) => void;

    api.notificationSettings.mockReturnValue(
      new Promise<Settings>((done) => {
        resolve = done;
      }),
    );
    const first = render(<PreferencesDialog onClose={vi.fn()} />);
    first.unmount();
    await act(async () => {
      resolve(defaults);
      await Promise.resolve();
    });

    api.notificationSettings.mockResolvedValue(defaults);
    api.saveNotificationSettings.mockReturnValue(
      new Promise<Settings>((done) => {
        resolve = done;
      }),
    );
    const second = render(<PreferencesDialog onClose={vi.fn()} />);

    fireEvent.click(await screen.findByRole("switch", { name: "Save to Telegram" }));
    second.unmount();
    await act(async () => {
      resolve({ ...defaults, telegramEnabled: false });
      await Promise.resolve();
    });
  });
});

function PreferencesDialog({ onClose }: { onClose: () => void }) {
  const cache = useNotificationPreferences(true);
  const [isOpen, setIsOpen] = useState(true);

  return (
    <>
      <button
        onClick={() => {
          setIsOpen(true);
        }}
      >
        Reopen
      </button>
      {isOpen && (
        <NotificationSettings
          cache={cache}
          onClose={() => {
            setIsOpen(false);
            onClose();
          }}
        />
      )}
    </>
  );
}
