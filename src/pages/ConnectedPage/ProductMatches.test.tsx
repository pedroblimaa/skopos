import { render } from "../../test-setup";
import { cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ProductMatches } from "./ProductMatches";
import type { ProductMatch } from "../../promotion.model";
const api = vi.hoisted(() => ({ openPromotionLink: vi.fn() }));
vi.mock("../../telegram", () => ({ telegram: api }));
beforeEach(() => {
  vi.resetAllMocks();
  api.openPromotionLink.mockResolvedValue(undefined);
});

afterEach(cleanup);
const match: ProductMatch = {
  watchId: 1,
  priceCents: 20100,
  message: {
    chatId: "channel:1",
    chatTitle: "Deals",
    messageId: 7,
    postedAt: 1700000000,
    messageLink: "https://t.me/deals/7",
    text: "\n🔥 Controller Ultimate Blue\nCoupon: SAVE\nhttps://shop.example/item",
  },
};

it.each([true, false])(
  "opens message links in the browser and reports failures: %s",
  async (success) => {
    if (!success) api.openPromotionLink.mockRejectedValue(new Error("Unavailable"));
    render(<ProductMatches name="Controller" matches={[match]} />);
    const link = screen.getByRole("link", { name: "https://shop.example/item", hidden: true });

    fireEvent.click(link);
    expect(api.openPromotionLink).toHaveBeenCalledWith("https://shop.example/item");
    if (!success) {
      expect(await screen.findByRole("alert")).toHaveTextContent("Could not open the link");
    }
  },
);

it("renders saved photos inside the full promotion and tolerates older messages without images", () => {
  const image = "data:image/jpeg;base64,/9j/2Q==";
  render(
    <ProductMatches
      name="Controller"
      matches={[{ ...match, message: { ...match.message, image } }]}
    />,
  );
  const photo = screen.getByRole("img", { hidden: true });
  expect(photo).toHaveAttribute("src", image);
  expect(photo).toHaveAttribute("alt", "Promotion image from Deals");
  expect(photo.closest(".product-match-content")).toBeInTheDocument();

  cleanup();
  render(<ProductMatches name="Controller" matches={[match]} />);
  expect(screen.queryByRole("img", { hidden: true })).not.toBeInTheDocument();
});

it("shows a description preview and keeps the full message in its own disclosure", () => {
  const { container } = render(<ProductMatches name="Controller" matches={[match]} />);
  const outer = container.querySelector(".product-matches");
  const detail = container.querySelector(".product-match-details");

  expect(outer).not.toHaveAttribute("open");
  expect(detail).not.toHaveAttribute("open");
  expect(container.querySelector(".product-match-preview")).toHaveTextContent(
    "🔥 Controller Ultimate Blue",
  );
  expect(container.querySelector(".product-match-preview")).not.toHaveTextContent("Coupon");
  expect(container.querySelector(".product-match-text")).toHaveTextContent("Coupon: SAVE");
  expect(screen.getByText("Deals ·", { exact: false })).toBeInTheDocument();
});

it.each([true, false])(
  "copies the original link without opening the promotion; success=%s",
  async (success) => {
    const writeText = vi.fn();
    if (success) writeText.mockResolvedValue(undefined);
    else writeText.mockRejectedValue(new Error("Denied"));
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const { container } = render(<ProductMatches name="Controller" matches={[match]} />);

    fireEvent.click(screen.getByRole("button", { name: "Copy message link" }));
    if (success) await screen.findByRole("button", { name: "Link copied" });
    else expect(await screen.findByRole("alert")).toHaveTextContent("Could not copy");

    expect(writeText).toHaveBeenCalledWith(match.message.messageLink);
    expect(container.querySelector(".product-match-details")).not.toHaveAttribute("open");
  },
);

it("handles an unknown price, absent link, and empty saved results", () => {
  render(
    <ProductMatches
      name="Controller"
      matches={[
        { ...match, priceCents: null, message: { ...match.message, messageLink: null, text: "" } },
      ]}
    />,
  );
  expect(screen.getByText("Price unavailable")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Copy message link" })).not.toBeInTheDocument();

  cleanup();
  render(<ProductMatches name="Controller" matches={[]} />);
  expect(screen.getByText("No saved matches for this product yet.")).toBeInTheDocument();
});
