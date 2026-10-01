import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Link, MemoryRouter, Route, Routes } from "react-router-dom";
import { AddWatchPage } from "./AddWatchPage";
import type { Watch } from "../../watch.model";

const api = vi.hoisted(() => ({
  createWatch: vi.fn(),
  updateWatch: vi.fn(),
  listWatches: vi.fn(),
}));
vi.mock("../../telegram", () => ({
  telegram: api,
  errorMessage: (error: unknown) => String(error),
}));

beforeEach(() => {
  vi.resetAllMocks();
  api.createWatch.mockResolvedValue({
    id: 1,
    phrases: ["Laptop Vivobook S14"],
    maxPriceCents: null,
  });
});
afterEach(cleanup);

function renderPage(path = "/watches/new") {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/watches/new" element={<AddWatchPage />} />
        <Route path="/watches/:watchId/edit" element={<AddWatchPage />} />
        <Route
          path="/connected"
          element={
            <>
              <h1>Products home</h1>
              <Link to="/watches/new">Add another product</Link>
            </>
          }
        />
      </Routes>
    </MemoryRouter>,
  );
}

describe("AddWatchPage", () => {
  it.each([
    ["create", true],
    ["create", false],
    ["update", true],
    ["update", false],
  ] as const)(
    "preserves a new draft after a late %s save result (success: %s)",
    async (kind, succeeds) => {
      let resolve!: () => void;
      let reject!: (reason: Error) => void;
      const pendingSave = new Promise<void>((res, rej) => {
        resolve = res;
        reject = rej;
      });
      api.listWatches.mockResolvedValue([{ id: 7, phrases: ["Laptop"], maxPriceCents: null }]);
      const saveCommand = kind === "create" ? api.createWatch : api.updateWatch;
      saveCommand.mockReturnValueOnce(pendingSave);
      renderPage(kind === "create" ? "/watches/new" : "/watches/7/edit");
      await screen.findByLabelText("Product name");

      fireEvent.change(screen.getByLabelText("Product name"), {
        target: { value: "First product" },
      });
      fireEvent.click(
        screen.getByRole("button", { name: kind === "create" ? "Add product" : "Save changes" }),
      );

      expect(screen.getByRole("button", { name: "Saving…" })).toBeDisabled();
      expect(saveCommand).toHaveBeenCalledTimes(1);

      fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
      fireEvent.click(screen.getByRole("link", { name: "Add another product" }));
      fireEvent.change(screen.getByLabelText("Product name"), {
        target: { value: "Second product" },
      });

      await act(async () => {
        if (succeeds) resolve();
        else reject(new Error("Late failure"));
        await pendingSave.catch(() => {});
      });

      expect(screen.getByLabelText("Product name")).toHaveValue("Second product");
      expect(screen.getByRole("button", { name: "Add product" })).toBeEnabled();
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    },
  );

  it("validates alternative names and preserves remaining names when removing a middle row", async () => {
    renderPage();
    fireEvent.change(screen.getByLabelText("Product name"), { target: { value: "Laptop" } });
    fireEvent.click(screen.getByRole("button", { name: "Add alternative name" }));
    fireEvent.click(screen.getByRole("button", { name: "Add alternative name" }));
    fireEvent.change(screen.getByLabelText("Alternative name 2"), {
      target: { value: "Asus S14" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add product" }));

    expect(screen.getByLabelText("Alternative name 1")).toHaveAttribute("aria-invalid", "true");
    expect(api.createWatch).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Remove alternative name 1" }));

    expect(screen.getByLabelText("Alternative name 1")).toHaveValue("Asus S14");

    fireEvent.click(screen.getByRole("button", { name: "Add alternative name" }));
    fireEvent.change(screen.getByLabelText("Alternative name 2"), {
      target: { value: "Vivobook" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add product" }));
    await waitFor(() => {
      expect(api.createWatch).toHaveBeenCalledWith({
        phrases: ["Laptop", "Asus S14", "Vivobook"],
        maxPriceCents: null,
      });
    });
  });

  it("loads a product, updates its ordered names and clears its price ceiling", async () => {
    api.listWatches.mockResolvedValue([
      { id: 7, phrases: ["Laptop", "Asus"], maxPriceCents: 350001 },
    ]);
    api.updateWatch.mockResolvedValue({
      id: 7,
      phrases: ["Laptop OLED", "Vivobook"],
      maxPriceCents: null,
    });

    renderPage("/watches/7/edit");

    expect(screen.getByRole("status")).toHaveTextContent("Loading product");
    expect(await screen.findByLabelText("Product name")).toHaveValue("Laptop");
    expect(screen.getByLabelText("Alternative name 1")).toHaveValue("Asus");
    expect(screen.getByLabelText(/Maximum price/)).toHaveValue("3500,01");

    fireEvent.change(screen.getByLabelText("Product name"), { target: { value: "Laptop OLED" } });
    fireEvent.change(screen.getByLabelText("Alternative name 1"), {
      target: { value: "Vivobook" },
    });
    fireEvent.change(screen.getByLabelText(/Maximum price/), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await screen.findByRole("heading", { name: "Products home" });
    expect(api.updateWatch).toHaveBeenCalledWith(7, {
      phrases: ["Laptop OLED", "Vivobook"],
      maxPriceCents: null,
    });

    expect(api.createWatch).not.toHaveBeenCalled();
  });

  it("preserves edited values on failure and disables fields while saving", async () => {
    api.listWatches.mockResolvedValue([{ id: 7, phrases: ["Laptop"], maxPriceCents: null }]);
    let reject!: (reason: Error) => void;
    api.updateWatch.mockReturnValueOnce(
      new Promise((_resolve, rej) => {
        reject = rej;
      }),
    );
    renderPage("/watches/7/edit");
    await screen.findByLabelText("Product name");
    expect(screen.getByLabelText(/Maximum price/)).toHaveValue("");

    fireEvent.change(screen.getByLabelText("Product name"), { target: { value: "Laptop OLED" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    expect(screen.getByRole("button", { name: "Saving…" })).toBeDisabled();
    expect(screen.getByLabelText("Product name")).toBeDisabled();

    await act(async () => {
      reject(new Error("Disk full"));
      await Promise.resolve();
    });

    expect(screen.getByRole("alert")).toHaveTextContent("Disk full");
    expect(screen.getByLabelText("Product name")).toHaveValue("Laptop OLED");

    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await screen.findByRole("heading", { name: "Products home" });
    expect(api.updateWatch).toHaveBeenCalledTimes(2);
  });

  it.each(["missing", "failure"])(
    "shows %s edit errors and offers the Products breadcrumb",
    async (kind) => {
      if (kind === "missing") api.listWatches.mockResolvedValue([]);
      else api.listWatches.mockRejectedValue(new Error("Storage unavailable"));
      renderPage("/watches/7/edit");

      expect(await screen.findByRole("alert")).toHaveTextContent(
        kind === "missing" ? "This product no longer exists." : "Storage unavailable",
      );
      expect(screen.queryByLabelText("Product name")).not.toBeInTheDocument();

      fireEvent.click(screen.getByRole("link", { name: "Products" }));

      expect(screen.getByRole("heading", { name: "Products home" })).toBeInTheDocument();
    },
  );

  it.each([true, false])("ignores a late edit load %s result", async (succeeds) => {
    let resolve!: (value: Watch[]) => void;
    let reject!: (reason: Error) => void;
    api.listWatches.mockReturnValue(
      new Promise<Watch[]>((res, rej) => {
        resolve = res;
        reject = rej;
      }),
    );
    renderPage("/watches/7/edit");
    fireEvent.click(screen.getByRole("link", { name: "Products" }));
    await act(async () => {
      if (succeeds) resolve([]);
      else reject(new Error("Late failure"));
      await Promise.resolve();
    });

    expect(screen.getByRole("heading", { name: "Products home" })).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("validates missing phrases and invalid prices", () => {
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Add product" }));

    expect(screen.getByText("Enter a product name.")).toBeInTheDocument();
    expect(api.createWatch).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("Product name"), {
      target: { value: "Laptop Vivobook S14" },
    });
    fireEvent.change(screen.getByLabelText(/Maximum price/), { target: { value: "0" } });

    expect(screen.getByText(/Enter a valid price/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add product" })).toBeDisabled();
  });

  it("adds an alternative name and saves ordered names", async () => {
    renderPage();
    fireEvent.change(screen.getByLabelText("Product name"), {
      target: { value: " Laptop Vivobook S14 " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add alternative name" }));
    fireEvent.change(screen.getByLabelText("Alternative name 1"), {
      target: { value: "Asus Vivobook 14" },
    });
    fireEvent.change(screen.getByLabelText(/Maximum price/), { target: { value: "3.500,00" } });
    fireEvent.click(screen.getByRole("button", { name: "Add product" }));

    await waitFor(() => {
      expect(api.createWatch).toHaveBeenCalledWith({
        phrases: ["Laptop Vivobook S14", "Asus Vivobook 14"],
        maxPriceCents: 350000,
      });
    });

    expect(await screen.findByRole("heading", { name: "Products home" })).toBeInTheDocument();
  });

  it("removes an added phrase", async () => {
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Add alternative name" }));
    fireEvent.click(screen.getByRole("button", { name: "Remove alternative name 1" }));
    fireEvent.change(screen.getByLabelText("Product name"), { target: { value: "RTX 5070" } });
    fireEvent.click(screen.getByRole("button", { name: "Add product" }));
    await waitFor(() => {
      expect(api.createWatch).toHaveBeenCalledWith({ phrases: ["RTX 5070"], maxPriceCents: null });
    });
  });

  it("keeps the form after a save error and allows retry", async () => {
    api.createWatch.mockRejectedValueOnce(new Error("Disk full"));

    renderPage();
    fireEvent.change(screen.getByLabelText("Product name"), { target: { value: "RTX 5070" } });
    fireEvent.click(screen.getByRole("button", { name: "Add product" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Disk full"));
    expect(screen.getByLabelText("Product name")).toHaveValue("RTX 5070");

    fireEvent.click(screen.getByRole("button", { name: "Add product" }));
    await waitFor(() => {
      expect(api.createWatch).toHaveBeenCalledTimes(2);
    });
  });

  it("cancels back to the watches home", () => {
    renderPage();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.getByRole("heading", { name: "Products home" })).toBeInTheDocument();
  });
});
