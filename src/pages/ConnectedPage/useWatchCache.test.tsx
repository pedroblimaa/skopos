import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useWatchCache } from "./useWatchCache";
import { useWatches } from "./watch-context";
import { render } from "../../test-setup";
import type { Watch } from "../../watch.model";

const api = vi.hoisted(() => ({ listWatches: vi.fn() }));
vi.mock("../../telegram", () => ({ telegram: api }));
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

it("loads once, shares pending reads, and keeps changes without rereading storage", async () => {
  const first = { id: 1, phrases: ["Laptop"], maxPriceCents: null };
  api.listWatches.mockResolvedValue([first]);
  const { result } = renderHook(useWatchCache);

  await act(async () => {
    const pending = result.current.load();
    expect(result.current.load()).toBe(pending);
    await pending;
  });
  expect(result.current.isLoaded).toBe(true);

  act(() => {
    result.current.update({ ...first, phrases: ["Laptop OLED"] });
    result.current.update({ id: 2, phrases: ["Controller"], maxPriceCents: 20100 });
  });
  expect(await result.current.load()).toEqual([
    { id: 2, phrases: ["Controller"], maxPriceCents: 20100 },
    { ...first, phrases: ["Laptop OLED"] },
  ]);

  act(() => {
    result.current.remove(1);
  });
  expect(result.current.watches.map((watch) => watch.id)).toEqual([2]);
  expect(api.listWatches).toHaveBeenCalledTimes(1);
});

it("retries a failed initial read and caches an empty list", async () => {
  api.listWatches.mockRejectedValueOnce(new Error("Storage unavailable")).mockResolvedValue([]);
  const { result } = renderHook(useWatchCache);

  await expect(result.current.load()).rejects.toThrow("Storage unavailable");
  expect(result.current.isLoaded).toBe(false);

  await act(async () => {
    await result.current.load();
  });
  await result.current.load();
  expect(result.current.isLoaded).toBe(true);
  expect(api.listWatches).toHaveBeenCalledTimes(2);
});

it("reloads an initial snapshot when a product is saved while it is pending", async () => {
  let finish!: (value: Watch[]) => void;
  const created = { id: 1, phrases: ["Controller"], maxPriceCents: null };
  api.listWatches
    .mockReturnValueOnce(
      new Promise<Watch[]>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValue([created]);
  const { result } = renderHook(useWatchCache);
  let pending!: Promise<Watch[]>;

  act(() => {
    pending = result.current.load();
  });
  act(() => {
    result.current.update(created);
  });

  await act(async () => {
    finish([]);
    await pending;
  });

  expect(result.current.watches).toEqual([created]);
  expect(await result.current.load()).toEqual([created]);
  expect(api.listWatches).toHaveBeenCalledTimes(2);
});

it("ignores late reads and mutations after unmount", async () => {
  let finish!: (value: []) => void;
  api.listWatches.mockReturnValue(
    new Promise<[]>((resolve) => {
      finish = resolve;
    }),
  );
  const { result, unmount } = renderHook(useWatchCache);
  const cache = result.current;
  const pending = cache.load();

  unmount();
  finish([]);
  await pending;
  cache.update({ id: 1, phrases: ["Laptop"], maxPriceCents: null });
  cache.remove(1);
  expect(cache.watches).toEqual([]);
});

it("requires the authenticated product cache", () => {
  function Consumer() {
    useWatches();
    return null;
  }
  expect(() => render(<Consumer />)).toThrow("authenticated watch cache");
});
