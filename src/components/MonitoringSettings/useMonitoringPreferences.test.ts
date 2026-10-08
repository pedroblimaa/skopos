import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useMonitoringPreferences } from "./useMonitoringPreferences";

const api = vi.hoisted(() => ({
  monitoringStatus: vi.fn(),
  startupSettings: vi.fn(),
  onMonitoringStatus: vi.fn(),
}));
vi.mock("../../telegram", () => ({ telegram: api }));
const status = {
  accountId: 77,
  enabled: true,
  isRunning: false,
  lastAttempt: null,
  nextDue: null,
  failure: null,
};
const startup = { enabled: true, failure: null };

beforeEach(() => {
  vi.resetAllMocks();
  api.monitoringStatus.mockResolvedValue(status);
  api.startupSettings.mockResolvedValue(startup);
  api.onMonitoringStatus.mockResolvedValue(vi.fn());
});
afterEach(cleanup);

it("does not load or subscribe before authentication", () => {
  renderHook(() => useMonitoringPreferences(false));

  expect(api.monitoringStatus).not.toHaveBeenCalled();
  expect(api.startupSettings).not.toHaveBeenCalled();
  expect(api.onMonitoringStatus).not.toHaveBeenCalled();
});

it("loads local records independently and preserves them across rerenders", async () => {
  api.monitoringStatus.mockReturnValue(new Promise(() => {}));
  const view = renderHook(() => useMonitoringPreferences(true));

  await waitFor(() => {
    expect(view.result.current.startup).toEqual(startup);
  });
  view.rerender();

  expect(view.result.current.status).toBeNull();
  expect(api.startupSettings).toHaveBeenCalledOnce();
  expect(api.monitoringStatus).toHaveBeenCalledOnce();
});

it("preserves newer events when the initial status resolves late", async () => {
  let finish!: (value: typeof status) => void;
  let receive!: (value: typeof status) => void;
  api.monitoringStatus.mockReturnValue(
    new Promise<typeof status>((resolve) => {
      finish = resolve;
    }),
  );
  api.onMonitoringStatus.mockImplementation((callback: typeof receive) => {
    receive = callback;

    return Promise.resolve(vi.fn());
  });
  const view = renderHook(() => useMonitoringPreferences(true));

  await act(async () => {
    receive({ ...status, isRunning: true });
    finish(status);
    await Promise.resolve();
  });

  expect(view.result.current.status?.isRunning).toBe(true);
});

it("disposes authenticated data and ignores late reads and events after sign-out", async () => {
  let finish!: (value: typeof startup) => void;
  let receive!: (value: typeof status) => void;
  const unsubscribe = vi.fn();
  api.startupSettings.mockReturnValue(
    new Promise<typeof startup>((resolve) => {
      finish = resolve;
    }),
  );
  api.onMonitoringStatus.mockImplementation((callback: typeof receive) => {
    receive = callback;

    return Promise.resolve(unsubscribe);
  });
  const view = renderHook(({ enabled }) => useMonitoringPreferences(enabled), {
    initialProps: { enabled: true },
  });
  await waitFor(() => {
    expect(view.result.current.status).toEqual(status);
  });

  view.rerender({ enabled: false });
  await act(async () => {
    finish(startup);
    receive({ ...status, isRunning: true });
    await Promise.resolve();
  });

  expect(view.result.current.startup).toBeNull();
  expect(view.result.current.status).toBeNull();
  expect(unsubscribe).toHaveBeenCalledOnce();
});

it("retries a failed read and clears its error", async () => {
  api.startupSettings.mockRejectedValueOnce({ code: "monitoringStorage" });
  const view = renderHook(() => useMonitoringPreferences(true));
  await waitFor(() => {
    expect(view.result.current.error).toEqual({ code: "monitoringStorage" });
  });

  act(() => {
    view.result.current.reload();
  });

  await waitFor(() => {
    expect(view.result.current.startup).toEqual(startup);
  });
  expect(view.result.current.error).toBeNull();
  expect(api.startupSettings).toHaveBeenCalledTimes(2);
});

it("retains a saved status when an older initial read fails", async () => {
  let reject!: (reason: unknown) => void;
  api.monitoringStatus.mockReturnValue(
    new Promise((_, fail) => {
      reject = fail;
    }),
  );
  const view = renderHook(() => useMonitoringPreferences(true));

  await act(async () => {
    view.result.current.updateStatus({ ...status, isRunning: true });
    reject({ code: "monitoringStorage" });
    await Promise.resolve();
  });

  expect(view.result.current.status?.isRunning).toBe(true);
  expect(view.result.current.error).toBeNull();
});

it("ignores monitoring events from another account", async () => {
  let receive!: (value: typeof status) => void;
  api.onMonitoringStatus.mockImplementation((callback: typeof receive) => {
    receive = callback;

    return Promise.resolve(vi.fn());
  });
  const view = renderHook(() => useMonitoringPreferences(true));
  await waitFor(() => {
    expect(view.result.current.status).toEqual(status);
  });

  act(() => {
    receive({ ...status, accountId: 88, isRunning: true });
  });

  expect(view.result.current.status).toEqual(status);
});
