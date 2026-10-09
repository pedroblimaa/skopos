import { render } from "../../test-setup";
import { act, cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Dialog } from "./Dialog";

beforeEach(() => {
  vi.useFakeTimers();
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.setAttribute("open", "");
      },
    },
    close: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.removeAttribute("open");
      },
    },
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
});

it("keeps the dialog mounted and inert through its exit animation, then closes once", () => {
  const onClose = vi.fn();
  render(
    <Dialog title="Preview" isBusy={false} onClose={onClose}>
      Content
    </Dialog>,
  );

  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));

  expect(screen.getByRole("dialog")).toHaveAttribute("data-closing", "true");
  expect(document.querySelector(".dialog-content")).toHaveAttribute("inert");
  expect(onClose).not.toHaveBeenCalled();

  act(() => {
    vi.advanceTimersByTime(179);
  });

  expect(onClose).not.toHaveBeenCalled();

  act(() => {
    vi.advanceTimersByTime(1);
  });

  expect(onClose).toHaveBeenCalledOnce();
});

it("blocks Escape while busy and cancels a pending close after unmount", () => {
  const onClose = vi.fn();
  const view = render(
    <Dialog title="Preview" isBusy={true} onClose={onClose}>
      Content
    </Dialog>,
  );

  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
  act(() => {
    vi.runAllTimers();
  });

  expect(onClose).not.toHaveBeenCalled();
  expect(screen.getByRole("dialog")).toHaveAttribute("data-closing", "false");

  view.rerender(
    <Dialog title="Preview" isBusy={false} onClose={onClose}>
      Content
    </Dialog>,
  );
  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
  view.unmount();
  act(() => {
    vi.runAllTimers();
  });

  expect(onClose).not.toHaveBeenCalled();
});

it("closes immediately when reduced motion is requested", () => {
  vi.stubGlobal("matchMedia", () => ({ matches: true }));
  const onClose = vi.fn();
  render(
    <Dialog title="Preview" isBusy={false} onClose={onClose}>
      Content
    </Dialog>,
  );

  fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));

  expect(onClose).toHaveBeenCalledOnce();
  expect(screen.getByRole("dialog")).toHaveAttribute("data-closing", "false");
});
