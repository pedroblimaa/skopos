import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { Outlet } from "react-router-dom";
import App from "./App";

vi.mock("./pages/LoginPage/LoginPage", () => ({
  LoginPage: () => <h1>Login view</h1>,
}));
vi.mock("./pages/ConnectedPage/ConnectedPage", () => ({
  ConnectedPage: () => <h1>Connected view</h1>,
}));
vi.mock("./pages/AddWatchPage/AddWatchPage", () => ({
  AddWatchPage: () => <h1>Add watch view</h1>,
}));

vi.mock("./components/AppShell/AppShell", () => ({ AppShell: () => <Outlet /> }));

afterEach(cleanup);

it.each([
  ["/", "Login view"],
  ["/login", "Login view"],
  ["/connected", "Connected view"],
  ["/watches/new", "Add watch view"],
  ["/watches/12/edit", "Add watch view"],
  ["/unknown", "Login view"],
])("routes %s to %s", (path, heading) => {
  window.history.replaceState({}, "", path);
  render(<App />);
  expect(screen.getByRole("heading", { name: heading })).toBeInTheDocument();
});
