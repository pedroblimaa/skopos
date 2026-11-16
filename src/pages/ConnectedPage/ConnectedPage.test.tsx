import { render } from "../../test-setup";
import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Link, MemoryRouter, Outlet, Route, Routes } from "react-router-dom";
import { ConnectedPage } from "./ConnectedPage";
import { SearchContext } from "./search-context";
import { useProductSearch } from "./useProductSearch";
import { WatchContext } from "./watch-context";
import { useWatchCache } from "./useWatchCache";
import type { Watch } from "../../watch.model";
import { AddWatchPage } from "../AddWatchPage/AddWatchPage";

const api = vi.hoisted(() => ({
  monitoringStatus: vi.fn(),
  onMonitoringStatus: vi.fn(),
  onSearchUpdated: vi.fn(),
  listWatches: vi.fn(),
  deleteWatch: vi.fn(),
  loadSearchResults: vi.fn(),
  searchProducts: vi.fn(),
  clearSearchResults: vi.fn(),
  createWatch: vi.fn(),
}));
vi.mock("../../telegram", () => ({
  telegram: api,
}));
const products: Watch[] = [
  {
    id: 1,
    phrases: ["Laptop Vivobook S14", "Asus Vivobook 14"],
    maxPriceCents: 350000,
    minPriceCents: null,
  },
  { id: 2, phrases: ["RTX 5070"], maxPriceCents: null, minPriceCents: null },
];

beforeEach(() => {
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value: vi.fn(function (this: HTMLDialogElement) {
        this.setAttribute("open", "");
      }),
    },
    close: {
      configurable: true,
      value: vi.fn(function (this: HTMLDialogElement) {
        this.removeAttribute("open");
      }),
    },
  });
  vi.resetAllMocks();
  api.monitoringStatus.mockResolvedValue({
    accountId: 77,
    enabled: true,
    isRunning: false,
    lastAttempt: null,
    nextDue: null,
    failure: null,
  });
  api.onMonitoringStatus.mockResolvedValue(vi.fn());
  api.onSearchUpdated.mockResolvedValue(vi.fn());
  api.listWatches.mockResolvedValue(products);
  api.deleteWatch.mockResolvedValue(undefined);
  api.loadSearchResults.mockResolvedValue({ matches: [], summary: null });
});
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
});

function SearchPage() {
  const search = useProductSearch();
  const watches = useWatchCache();

  return (
    <SearchContext.Provider value={search}>
      <WatchContext.Provider value={watches}>
        <ConnectedPage />
      </WatchContext.Provider>
    </SearchContext.Provider>
  );
}

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/connected"]}>
      <Routes>
        <Route path="/connected" element={<SearchPage />} />
        <Route path="/watches/new" element={<h1>Add product</h1>} />
        <Route path="/watches/:watchId/edit" element={<h1>Edit product</h1>} />
      </Routes>
    </MemoryRouter>,
  );
}

function CachedSession() {
  const search = useProductSearch();
  const watches = useWatchCache();

  return (
    <SearchContext.Provider value={search}>
      <WatchContext.Provider value={watches}>
        <Link to="/connected">Products tab</Link>
        <Link to="/chats">Chats tab</Link>
        <Outlet />
      </WatchContext.Provider>
    </SearchContext.Provider>
  );
}

it("shows a product saved after navigating away from a pending initial list", async () => {
  let finish!: (value: Watch[]) => void;
  const created = { id: 3, phrases: ["Controller"], maxPriceCents: null, minPriceCents: null };
  api.listWatches
    .mockReturnValueOnce(
      new Promise<Watch[]>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValue([created, ...products]);

  api.createWatch.mockResolvedValue(created);
  render(
    <MemoryRouter initialEntries={["/connected"]}>
      <Routes>
        <Route element={<CachedSession />}>
          <Route path="/connected" element={<ConnectedPage />} />
          <Route path="/watches/new" element={<AddWatchPage />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );

  fireEvent.click(screen.getByRole("button", { name: "Add product" }));
  fireEvent.change(screen.getByLabelText("Product name"), { target: { value: "Controller" } });
  fireEvent.click(screen.getByRole("button", { name: "Add product" }));
  await screen.findByRole("heading", { name: "Products" });

  await act(async () => {
    finish(products);
    await Promise.resolve();
  });

  expect(await screen.findByRole("link", { name: "Controller" })).toBeInTheDocument();
  expect(api.createWatch).toHaveBeenCalledTimes(1);
});

it("returns from Chats with cached products and results without a loading state", async () => {
  render(
    <MemoryRouter initialEntries={["/connected"]}>
      <Routes>
        <Route element={<CachedSession />}>
          <Route path="/connected" element={<ConnectedPage />} />
          <Route path="/chats" element={<h1>Chats page</h1>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
  await screen.findByRole("link", { name: "RTX 5070" });
  await waitFor(() => expect(screen.getByRole("status")).toBeEmptyDOMElement());

  fireEvent.click(screen.getByRole("link", { name: "Chats tab" }));

  expect(screen.getByRole("heading", { name: "Chats page" })).toBeInTheDocument();

  fireEvent.click(screen.getByRole("link", { name: "Products tab" }));

  expect(screen.getByRole("link", { name: "RTX 5070" })).toBeInTheDocument();
  expect(screen.queryByText(/Loading products|Loading saved matches/)).not.toBeInTheDocument();
  expect(api.listWatches).toHaveBeenCalledTimes(1);
  expect(api.loadSearchResults).toHaveBeenCalledTimes(1);
});

it("opens deletion as a modal, cancels with Escape and blocks dismissal while deleting", async () => {
  let finish!: () => void;

  api.deleteWatch.mockReturnValue(
    new Promise<void>((resolve) => {
      finish = resolve;
    }),
  );
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: "Delete RTX 5070" }));
  const dialog = screen.getByRole("dialog", { name: "Delete RTX 5070" });

  expect(dialog).toHaveAttribute("open");
  expect(screen.getByRole("button", { name: "Keep product" })).toHaveFocus();

  fireEvent(dialog, new Event("cancel", { cancelable: true }));

  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  expect(api.deleteWatch).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Delete RTX 5070" }));
  fireEvent.click(screen.getByRole("button", { name: "Delete product" }));
  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));

  expect(screen.getByRole("dialog")).toHaveAttribute("aria-busy", "true");

  await act(async () => {
    finish();
    await Promise.resolve();
  });

  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(screen.queryByRole("link", { name: "RTX 5070" })).not.toBeInTheDocument();
});

describe("Products", () => {
  it("shows saved products, price ceilings, and a link to edit each product", async () => {
    renderPage();

    expect(screen.getByText(/Loading products/)).toBeInTheDocument();

    const laptop = await screen.findByRole("link", { name: "Laptop Vivobook S14" });

    expect(laptop).toHaveAttribute("href", "/watches/1/edit");
    expect(laptop.closest(".connected-watch-details")).toHaveTextContent(
      /2 names · Up to R\$\s*3\.500,00/,
    );
    expect(
      screen.getByRole("link", { name: "RTX 5070" }).closest(".connected-watch-details"),
    ).toHaveTextContent("1 name · Any price");

    fireEvent.click(laptop);

    expect(screen.getByRole("heading", { name: "Edit product" })).toBeInTheDocument();
  });

  it.each(["Add product", "Add your first product"])(
    "opens creation through %s",
    async (action) => {
      api.listWatches.mockResolvedValue([]);

      renderPage();
      await screen.findByText("What are you looking for?");

      fireEvent.click(screen.getByRole("button", { name: action }));

      expect(screen.getByRole("heading", { name: "Add product" })).toBeInTheDocument();
    },
  );

  it("reports a loading failure", async () => {
    api.listWatches.mockRejectedValue({ code: "watchStorage" });

    renderPage();

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not save or load products");
    expect(screen.queryByText("What are you looking for?")).not.toBeInTheDocument();
  });

  it("requires confirmation and can keep a product", async () => {
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Delete RTX 5070" }));

    expect(screen.getByText("Delete this product?")).toBeInTheDocument();
    expect(api.deleteWatch).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Keep product" }));

    expect(screen.queryByText("Delete this product?")).not.toBeInTheDocument();
    expect(screen.getByText("RTX 5070")).toBeInTheDocument();
  });

  it("deletes only the confirmed product and prevents duplicate requests", async () => {
    let finish!: () => void;

    api.deleteWatch.mockReturnValue(
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
    );
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Delete RTX 5070" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete product" }));

    expect(api.deleteWatch).toHaveBeenCalledWith(2);
    expect(screen.getByRole("button", { name: "Deleting…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Keep product" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Delete Laptop Vivobook S14" })).toBeDisabled();

    await act(async () => {
      finish();
      await Promise.resolve();
    });

    expect(screen.queryByText("RTX 5070")).not.toBeInTheDocument();
    expect(screen.getByText("Laptop Vivobook S14")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Delete Laptop Vivobook S14" }));

    api.deleteWatch.mockResolvedValue(undefined);
    fireEvent.click(screen.getByRole("button", { name: "Delete product" }));

    expect(await screen.findByText("What are you looking for?")).toBeInTheDocument();
  });

  it("preserves products after delete failure and allows retry", async () => {
    api.deleteWatch.mockRejectedValueOnce({ code: "watchStorage" });

    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Delete RTX 5070" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete product" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not save or load products");
    expect(screen.getByText("RTX 5070")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Delete product" }));
    await waitFor(() => expect(screen.queryByText("RTX 5070")).not.toBeInTheDocument());

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(api.deleteWatch).toHaveBeenCalledTimes(2);
  });

  it.each([true, false])("ignores a late list %s result", async (succeeds) => {
    let resolve!: (value: Watch[]) => void;
    let reject!: (reason: Error) => void;

    api.listWatches.mockReturnValue(
      new Promise<Watch[]>((res, rej) => {
        resolve = res;
        reject = rej;
      }),
    );
    const view = renderPage();
    view.unmount();
    await act(async () => {
      if (succeeds) resolve(products);
      else reject(new Error("Late failure"));
      await Promise.resolve();
    });

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
