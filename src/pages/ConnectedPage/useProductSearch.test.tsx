import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useProductSearch } from "./useProductSearch";
import type { SearchResults } from "../../promotion.model";

const api = vi.hoisted(() => ({
  loadSearchResults: vi.fn(),
  searchProducts: vi.fn(),
  clearSearchResults: vi.fn(),
}));
vi.mock("../../telegram", () => ({ telegram: api }));
const saved: SearchResults = {
  matches: [],
  summary: {
    startedAt: 200,
    since: 100,
    completedChats: 2,
    totalChats: 2,
    unavailableChats: [],
    failure: null,
  },
};
beforeEach(() => {
  vi.resetAllMocks();
  api.loadSearchResults.mockResolvedValue(saved);
  api.searchProducts.mockResolvedValue(saved);
  api.clearSearchResults.mockResolvedValue(undefined);
});
afterEach(cleanup);

it("reuses saved results and reloads only after product criteria change", async () => {
  const { result } = renderHook(useProductSearch);
  await act(async () => {
    await result.current.load();
  });

  await result.current.load();
  expect(api.loadSearchResults).toHaveBeenCalledTimes(1);

  act(() => {
    result.current.invalidate();
  });
  await act(async () => {
    await result.current.load();
  });
  expect(api.loadSearchResults).toHaveBeenCalledTimes(2);
});

it("does not treat a read started before invalidation as a current cache", async () => {
  let finish!: (value: SearchResults) => void;
  api.loadSearchResults.mockReturnValueOnce(
    new Promise<SearchResults>((resolve) => {
      finish = resolve;
    }),
  );
  const { result } = renderHook(useProductSearch);
  let pending!: Promise<boolean>;
  act(() => {
    pending = result.current.load();
    result.current.invalidate();
  });

  await act(async () => {
    finish(saved);
    await pending;
  });
  await act(async () => {
    await result.current.load();
  });
  expect(api.loadSearchResults).toHaveBeenCalledTimes(2);
});

it("loads saved results and retains them when a later search fails", async () => {
  const { result } = renderHook(useProductSearch);

  await act(async () => {
    await result.current.load();
  });
  expect(result.current.results).toEqual(saved);
  expect(result.current.isLoaded).toBe(true);

  api.searchProducts.mockRejectedValue({ code: "searchFailed" });
  await act(async () => {
    expect(await result.current.search()).toBe(false);
  });
  expect(result.current.results).toEqual(saved);
  expect(result.current.error).toEqual({ code: "searchFailed" });
  expect(result.current.isBusy).toBe(false);
});

it("rejects overlapping operations and reloads after cleanup", async () => {
  let finish!: (value: SearchResults) => void;
  api.searchProducts.mockReturnValue(
    new Promise<SearchResults>((resolve) => {
      finish = resolve;
    }),
  );
  const { result } = renderHook(useProductSearch);
  let pending!: Promise<boolean>;

  act(() => {
    pending = result.current.search();
  });
  expect(result.current.isSearching).toBe(true);
  await act(async () => {
    expect(await result.current.search()).toBe(false);
    expect(await result.current.clear(null)).toBe(false);
  });
  expect(api.searchProducts).toHaveBeenCalledTimes(1);
  expect(api.clearSearchResults).not.toHaveBeenCalled();

  await act(async () => {
    finish(saved);
    await pending;
  });
  await act(async () => {
    expect(await result.current.clear(150)).toBe(true);
  });
  expect(api.clearSearchResults).toHaveBeenCalledWith(150);
  expect(api.loadSearchResults).toHaveBeenCalledTimes(1);
});

it.each([true, false])("ignores a late search completion after unmount: %s", async (success) => {
  let finish!: (value: SearchResults) => void;
  let fail!: (reason: unknown) => void;
  api.searchProducts.mockReturnValue(
    new Promise<SearchResults>((resolve, reject) => {
      finish = resolve;
      fail = reject;
    }),
  );
  const { result, unmount } = renderHook(useProductSearch);
  let pending!: Promise<boolean>;
  act(() => {
    pending = result.current.search();
  });

  unmount();
  if (success) finish(saved);
  else fail({ code: "searchFailed" });

  expect(await pending).toBe(false);
  expect(await result.current.load()).toBe(false);
  expect(api.loadSearchResults).not.toHaveBeenCalled();
});
