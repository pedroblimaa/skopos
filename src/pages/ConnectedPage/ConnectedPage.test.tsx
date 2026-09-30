import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ConnectedPage } from "./ConnectedPage";
import type { Watch } from "../../watch.model";

const api = vi.hoisted(() => ({ listWatches: vi.fn(), deleteWatch: vi.fn() }));
vi.mock("../../telegram", () => ({
  telegram: api,
  errorMessage: (error: unknown) => String(error),
}));
const products: Watch[] = [
  { id: 1, phrases: ["Laptop Vivobook S14", "Asus Vivobook 14"], maxPriceCents: 350000 },
  { id: 2, phrases: ["RTX 5070"], maxPriceCents: null },
];
beforeEach(() => {
  vi.resetAllMocks();
  api.listWatches.mockResolvedValue(products);
  api.deleteWatch.mockResolvedValue(undefined);
});
afterEach(cleanup);

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/connected"]}>
      <Routes>
        <Route path="/connected" element={<ConnectedPage />} />
        <Route path="/watches/new" element={<h1>Add product</h1>} />
        <Route path="/watches/:watchId/edit" element={<h1>Edit product</h1>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("Products", () => {
  it("shows saved products, price ceilings, and a link to edit each product", async () => {
    renderPage();

    expect(screen.getByRole("status")).toHaveTextContent("Loading products");

    const laptop = await screen.findByRole("link", { name: /Laptop Vivobook S14/ });
    expect(laptop).toHaveAttribute("href", "/watches/1/edit");
    expect(laptop).toHaveTextContent(/2 names · Up to R\$\s*3\.500,00/);
    expect(screen.getByRole("link", { name: /RTX 5070/ })).toHaveTextContent("1 name · Any price");

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
    api.listWatches.mockRejectedValue(new Error("Storage unavailable"));

    renderPage();

    expect(await screen.findByRole("alert")).toHaveTextContent("Storage unavailable");
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
    api.deleteWatch.mockRejectedValueOnce(new Error("Disk busy"));

    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Delete RTX 5070" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete product" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Disk busy");
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
