import { cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { render } from "../../test-setup";
import { PromotionText } from "./PromotionText";

afterEach(cleanup);
it("preserves multiline text, duplicate URLs and punctuation without rendering message HTML", () => {
  const open = vi.fn();
  const text =
    "<b>Controller</b>\nhttps://shop.example/item?a=1&b=2.\nhttps://shop.example/item?a=1&b=2\nNo links here";
  const { container } = render(<PromotionText text={text} onOpenLink={open} />);
  const links = screen.getAllByRole("link");
  expect(links).toHaveLength(2);
  expect(container.getElementsByTagName("p")[0].textContent).toBe(text);
  expect(container.getElementsByTagName("b")).toHaveLength(0);
  expect(links[0]).toHaveAttribute("href", "https://shop.example/item?a=1&b=2");

  fireEvent.click(links[0]);
  expect(open).toHaveBeenCalledWith("https://shop.example/item?a=1&b=2");
});

it("leaves text without web links unchanged", () => {
  render(<PromotionText text="Controller\nCoupon: SAVE" onOpenLink={vi.fn()} />);
  expect(screen.queryByRole("link")).not.toBeInTheDocument();
});
