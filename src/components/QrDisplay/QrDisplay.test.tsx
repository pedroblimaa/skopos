import { render } from "../../test-setup";
import { act, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { QrDisplay } from "./QrDisplay";

afterEach(() => vi.useRealTimers());

it("shows the server expiry and replaces the QR when Telegram refreshes it", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-09-25T12:00:00Z"));
  const first = { url: "tg://login?token=first", expiresAt: Date.now() / 1000 + 30 };
  const view = render(<QrDisplay token={first} />);

  expect(screen.getByText("Expires in 00:30")).toBeInTheDocument();
  expect(view.container.querySelector("svg")).toBeInTheDocument();

  view.rerender(
    <QrDisplay token={{ url: "tg://login?token=second", expiresAt: Date.now() / 1000 + 90 }} />,
  );
  expect(screen.getByText("Expires in 01:30")).toBeInTheDocument();
  act(() => {
    vi.advanceTimersByTime(91_000);
  });
  expect(screen.getByText("Refreshing QR code…")).toBeInTheDocument();
});
