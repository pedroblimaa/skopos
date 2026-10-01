import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { InfoTooltip } from "./InfoTooltip";

afterEach(cleanup);

it("describes its trigger and supports Escape dismissal and reopening", () => {
  render(<InfoTooltip label="How names match">All words; any name.</InfoTooltip>);
  const trigger = screen.getByRole("button", { name: "How names match" });
  const tooltip = screen.getByRole("tooltip");
  expect(trigger).toHaveAttribute("aria-describedby", tooltip.id);
  expect(trigger).toHaveAccessibleDescription("All words; any name.");

  const wrapper = trigger.parentElement;
  if (!wrapper) throw new Error("Tooltip wrapper is missing");
  fireEvent.focus(trigger);
  fireEvent.keyDown(trigger, { key: "Enter" });

  expect(wrapper).toHaveAttribute("data-dismissed", "false");

  fireEvent.keyDown(trigger, { key: "Escape" });

  expect(wrapper).toHaveAttribute("data-dismissed", "true");

  fireEvent.mouseEnter(wrapper);

  expect(wrapper).toHaveAttribute("data-dismissed", "false");

  fireEvent.keyDown(trigger, { key: "Escape" });
  fireEvent.blur(trigger);
  fireEvent.focus(trigger);

  expect(wrapper).toHaveAttribute("data-dismissed", "false");
});

it("gives multiple explanations distinct accessible associations", () => {
  render(
    <>
      <InfoTooltip label="Names">Names help</InfoTooltip>
      <InfoTooltip label="Prices">Prices help</InfoTooltip>
    </>,
  );
  expect(screen.getByRole("button", { name: "Names" })).toHaveAccessibleDescription("Names help");
  expect(screen.getByRole("button", { name: "Prices" })).toHaveAccessibleDescription("Prices help");

  const tips = screen.getAllByRole("tooltip");
  expect(tips[0].id).not.toBe(tips[1].id);
});
