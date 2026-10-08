import { render } from "../../test-setup";
import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useMonitoringPreferences } from "./useMonitoringPreferences";
import { MonitoringSettings } from "./MonitoringSettings";
import type { MonitoringStatus } from "../../monitoring.model";

const api = vi.hoisted(() => ({
  monitoringStatus: vi.fn(),
  startupSettings: vi.fn(),
  saveMonitoringSettings: vi.fn(),
  saveStartupSettings: vi.fn(),
  onMonitoringStatus: vi.fn(),
}));
vi.mock("../../telegram", () => ({ telegram: api }));
const status: MonitoringStatus = {
  accountId: 77,
  enabled: true,
  isRunning: false,
  lastAttempt: null,
  nextDue: 1791381600,
  failure: null,
};

beforeEach(() => {
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.setAttribute("open", "");
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
  api.monitoringStatus.mockResolvedValue(status);
  api.startupSettings.mockResolvedValue({ enabled: true, failure: null });
  api.onMonitoringStatus.mockResolvedValue(vi.fn());
  api.saveMonitoringSettings.mockResolvedValue({ ...status, enabled: false });
  api.saveStartupSettings.mockResolvedValue({ enabled: false, failure: null });
});

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
});

it("shows the schedule and saves monitoring and startup independently", async () => {
  const onClose = vi.fn();

  render(<PreferencesDialog onClose={onClose} />);
  const monitoring = await screen.findByRole("switch", { name: "Automatic monitoring" });
  const startup = screen.getByRole("switch", { name: "Start with Windows" });

  expect(monitoring).toBeChecked();
  expect(startup).toBeChecked();
  expect(screen.getByText(/at the start of the day and at 18:00/)).toBeInTheDocument();

  fireEvent.click(monitoring);
  await waitFor(() => expect(monitoring).not.toBeChecked());

  expect(api.saveMonitoringSettings).toHaveBeenCalledWith(false);
  expect(startup).toBeChecked();
  expect(screen.getByText("Paused")).toBeInTheDocument();

  fireEvent.click(startup);
  await waitFor(() => expect(startup).not.toBeChecked());

  expect(api.saveStartupSettings).toHaveBeenCalledWith(false);

  fireEvent.click(screen.getByRole("button", { name: "Close" }));

  await waitFor(() => {
    expect(onClose).toHaveBeenCalledOnce();
  });
});

it("keeps the existing switch value when a save fails", async () => {
  api.saveStartupSettings.mockRejectedValue({ code: "startupFailed" });
  render(<PreferencesDialog onClose={vi.fn()} />);
  const startup = await screen.findByRole("switch", { name: "Start with Windows" });

  fireEvent.click(startup);

  expect(await screen.findByRole("alert")).toHaveTextContent("Could not update Windows startup");
  expect(startup).toBeChecked();
  expect(startup).toBeEnabled();
});

it("retains a newer status event when the initial read completes late and unsubscribes", async () => {
  let finish!: (value: MonitoringStatus) => void;
  let receive!: (value: MonitoringStatus) => void;
  const unsubscribe = vi.fn();

  api.monitoringStatus.mockReturnValue(
    new Promise<MonitoringStatus>((resolve) => {
      finish = resolve;
    }),
  );
  api.onMonitoringStatus.mockImplementation((callback: typeof receive) => {
    receive = callback;

    return Promise.resolve(unsubscribe);
  });
  const view = render(<PreferencesDialog onClose={vi.fn()} />);

  await act(async () => {
    receive({ ...status, isRunning: true });
    finish(status);
    await Promise.resolve();
  });

  expect(await screen.findByRole("status")).toHaveTextContent("Checking new messages");

  view.unmount();

  await act(async () => {
    await Promise.resolve();
  });

  expect(unsubscribe).toHaveBeenCalledOnce();
});

function PreferencesDialog({ onClose }: { onClose: () => void }) {
  const cache = useMonitoringPreferences(true);

  return <MonitoringSettings cache={cache} onClose={onClose} />;
}
