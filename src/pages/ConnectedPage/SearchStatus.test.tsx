import { render } from "../../test-setup";
import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { SearchStatus, SearchFeedback, SearchInformation } from "./SearchStatus";
import { ClearResultsDialog } from "./ClearResultsDialog";
import { useSearchResults } from "./search-context";
import type { useProductSearch } from "./useProductSearch";

type Search = ReturnType<typeof useProductSearch>;
function state(): Search {
  return {
    results: {
      matches: [],
      summary: {
        startedAt: 200,
        since: 100,
        completedChats: 2,
        totalChats: 2,
        unavailableChats: [],
        failure: null,
      },
    },
    error: null,
    isBusy: false,
    isSearching: false,
    isLoaded: true,
    load: vi.fn(),
    search: vi.fn(),
    clear: vi.fn().mockResolvedValue(true),
  };
}
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
});
afterEach(() => {
  cleanup();
  Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
});

it("orders cleanup, add, search and keeps successful search metadata in explanations", () => {
  const search = state();
  const { container } = render(
    <MemoryRouter>
      <SearchStatus search={search} isSearchDisabled={false} />
      <SearchInformation search={search} />
      <SearchFeedback search={search} />
    </MemoryRouter>,
  );

  const actions = container.querySelectorAll(".button");
  expect(Array.from(actions).map((button) => button.getAttribute("aria-label"))).toEqual([
    "Clear results",
    "Add product",
    "Search now",
  ]);
  expect(screen.getByRole("button", { name: "Search now" })).toHaveClass("button--primary");
  expect(screen.getByRole("button", { name: "Last search" })).toHaveAccessibleDescription(
    /Last search:/,
  );
  expect(screen.queryByText(/Search complete/)).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Search now" }));
  expect(search.search).toHaveBeenCalledTimes(1);
});

it("clears all saved results only after submitting the modal", async () => {
  const search = state();
  render(
    <MemoryRouter>
      <SearchStatus search={search} isSearchDisabled={false} />
    </MemoryRouter>,
  );

  fireEvent.click(screen.getByRole("button", { name: "Clear results" }));
  expect(screen.getByRole("dialog")).toBeInTheDocument();
  expect(search.clear).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Clear results" }));
  fireEvent.submit(screen.getByRole("dialog").getElementsByTagName("form")[0]);
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  expect(search.clear).toHaveBeenCalledWith(null);
});

it("sends the local cutoff as epoch seconds and reports cleanup failures", async () => {
  const clear = vi.fn().mockResolvedValue(false);
  const close = vi.fn();
  const { rerender } = render(
    <ClearResultsDialog isBusy={false} error={null} onClear={clear} onClose={close} />,
  );
  fireEvent.click(screen.getByLabelText("Messages posted before a date and time"));
  fireEvent.change(screen.getByLabelText("Message date and time (local)"), {
    target: { value: "2026-10-02T10:00" },
  });

  fireEvent.submit(screen.getByRole("dialog").getElementsByTagName("form")[0]);
  expect(await screen.findByRole("alert")).toHaveTextContent("Could not finish clearing");
  expect(clear).toHaveBeenCalledWith(new Date("2026-10-02T10:00").getTime() / 1000);
  expect(close).not.toHaveBeenCalled();

  rerender(
    <ClearResultsDialog
      isBusy={false}
      error={{ code: "searchStorage" }}
      onClear={clear}
      onClose={close}
    />,
  );
  expect(screen.getByRole("alert")).toHaveTextContent("Could not load or save search results");
});

it("rejects an invalid cutoff and keeps a busy dialog open on Escape", () => {
  const clear = vi.fn();
  const close = vi.fn();
  const { rerender } = render(
    <ClearResultsDialog isBusy={false} error={null} onClear={clear} onClose={close} />,
  );
  fireEvent.click(screen.getByLabelText("Messages posted before a date and time"));
  fireEvent.submit(screen.getByRole("dialog").getElementsByTagName("form")[0]);
  expect(clear).not.toHaveBeenCalled();

  rerender(<ClearResultsDialog isBusy error={null} onClear={clear} onClose={close} />);
  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
  expect(close).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "Clearing…" })).toBeDisabled();

  rerender(<ClearResultsDialog isBusy={false} error={null} onClear={clear} onClose={close} />);
  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
  expect(close).toHaveBeenCalledTimes(1);
});

it("shows actionable partial failures and a source-selection recovery link", () => {
  const search = state();
  search.results.summary = {
    startedAt: 200,
    since: 100,
    totalChats: 2,
    completedChats: 1,
    unavailableChats: ["Private deals"],
    failure: { code: "searchFailed" },
  };
  search.error = { code: "searchNeedsChats" };
  render(
    <MemoryRouter>
      <SearchFeedback search={search} />
    </MemoryRouter>,
  );

  expect(screen.getByRole("status")).toHaveTextContent("Search incomplete");
  expect(screen.getByText("Unavailable chats: Private deals")).toBeInTheDocument();
  expect(screen.getAllByRole("alert")).toHaveLength(2);
  expect(screen.getByRole("link", { name: "Choose chats" })).toHaveAttribute("href", "/chats");
});

it.each([true, false])("shows in-flight search or load status: searching=%s", (isSearching) => {
  const search = { ...state(), isBusy: true, isSearching };
  render(
    <MemoryRouter>
      <SearchStatus search={search} isSearchDisabled={false} />
      <SearchFeedback search={search} />
    </MemoryRouter>,
  );
  expect(screen.getByRole("status")).toHaveTextContent(
    isSearching ? "Searching" : "Loading saved matches",
  );
  expect(screen.getByRole("button", { name: "Search now" })).toBeDisabled();
});

it("rejects a results consumer outside the authenticated provider", () => {
  function Consumer() {
    useSearchResults();
    return null;
  }

  expect(() => render(<Consumer />)).toThrow("authenticated search context");
});
