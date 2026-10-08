import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useNotificationPreferences } from "./useNotificationPreferences";

const api = vi.hoisted(() => ({
  notificationSettings: vi.fn(),
  notificationStatus: vi.fn(),
  onNotificationStatus: vi.fn(),
}));
vi.mock("../../telegram", () => ({ telegram: api }));
const settings = { telegramEnabled: true, desktopEnabled: true, language: "en" as const };
const status = { pending: 0, uncertain: 0, failure: null };

beforeEach(() => {
  vi.resetAllMocks();
  api.notificationSettings.mockResolvedValue(settings);
  api.notificationStatus.mockResolvedValue(status);
  api.onNotificationStatus.mockResolvedValue(vi.fn());
});
afterEach(cleanup);

it("does not load or subscribe before authentication", () => {
  renderHook(() => useNotificationPreferences(false));

  expect(api.notificationStatus).not.toHaveBeenCalled();
  expect(api.notificationSettings).not.toHaveBeenCalled();
  expect(api.onNotificationStatus).not.toHaveBeenCalled();
});

it("loads local records independently and preserves them across rerenders", async () => {
  api.notificationStatus.mockReturnValue(new Promise(() => {}));
  const view = renderHook(() => useNotificationPreferences(true));

  await waitFor(() => {
    expect(view.result.current.settings).toEqual(settings);
  });
  view.rerender();

  expect(view.result.current.status).toBeNull();
  expect(api.notificationSettings).toHaveBeenCalledOnce();
  expect(api.notificationStatus).toHaveBeenCalledOnce();
});

it("preserves newer events when the initial status resolves late", async () => {
  let finish!: (value: typeof status) => void;
  let receive!: (value: typeof status) => void;
  api.notificationStatus.mockReturnValue(
    new Promise<typeof status>((resolve) => {
      finish = resolve;
    }),
  );
  api.onNotificationStatus.mockImplementation((callback: typeof receive) => {
    receive = callback;

    return Promise.resolve(vi.fn());
  });
  const view = renderHook(() => useNotificationPreferences(true));

  await act(async () => {
    receive({ ...status, pending: 3 });
    finish(status);
    await Promise.resolve();
  });

  expect(view.result.current.status?.pending).toBe(3);
});

it("disposes authenticated data and ignores late reads and events after sign-out", async () => {
  let finish!: (value: typeof settings) => void;
  let receive!: (value: typeof status) => void;
  const unsubscribe = vi.fn();
  api.notificationSettings.mockReturnValue(
    new Promise<typeof settings>((resolve) => {
      finish = resolve;
    }),
  );
  api.onNotificationStatus.mockImplementation((callback: typeof receive) => {
    receive = callback;

    return Promise.resolve(unsubscribe);
  });
  const view = renderHook(({ enabled }) => useNotificationPreferences(enabled), {
    initialProps: { enabled: true },
  });
  await waitFor(() => {
    expect(view.result.current.status).toEqual(status);
  });

  view.rerender({ enabled: false });
  await act(async () => {
    finish(settings);
    receive({ ...status, pending: 3 });
    await Promise.resolve();
  });

  expect(view.result.current.settings).toBeNull();
  expect(view.result.current.status).toBeNull();
  expect(unsubscribe).toHaveBeenCalledOnce();
});

it("retries a failed read and clears its error", async () => {
  api.notificationSettings.mockRejectedValueOnce({ code: "notificationStorage" });
  const view = renderHook(() => useNotificationPreferences(true));
  await waitFor(() => {
    expect(view.result.current.error).toEqual({ code: "notificationStorage" });
  });

  act(() => {
    view.result.current.reload();
  });

  await waitFor(() => {
    expect(view.result.current.settings).toEqual(settings);
  });
  expect(view.result.current.error).toBeNull();
  expect(api.notificationSettings).toHaveBeenCalledTimes(2);
});

it("retains a saved status when an older initial read fails", async () => {
  let reject!: (reason: unknown) => void;
  api.notificationStatus.mockReturnValue(
    new Promise((_, fail) => {
      reject = fail;
    }),
  );
  const view = renderHook(() => useNotificationPreferences(true));

  await act(async () => {
    view.result.current.updateStatus({ ...status, pending: 3 });
    reject({ code: "notificationStorage" });
    await Promise.resolve();
  });

  expect(view.result.current.status?.pending).toBe(3);
  expect(view.result.current.error).toBeNull();
});
