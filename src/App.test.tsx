import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("./pages/LoginPage/LoginPage", () => ({
  LoginPage: () => <h1>Login view</h1>,
}));
vi.mock("./pages/ConnectedPage/ConnectedPage", () => ({
  ConnectedPage: () => <h1>Connected view</h1>,
}));

afterEach(cleanup);

it.each([
  ["/", "Login view"],
  ["/login", "Login view"],
  ["/connected", "Connected view"],
  ["/unknown", "Login view"],
])("routes %s to %s", (path, heading) => {
  window.history.replaceState({}, "", path);
  render(<App />);
  expect(screen.getByRole("heading", { name: heading })).toBeInTheDocument();
});
