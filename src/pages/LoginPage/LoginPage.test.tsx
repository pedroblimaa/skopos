import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { LoginPage } from "./LoginPage";

const auth = vi.hoisted(() => ({
  status: vi.fn(),
  startQr: vi.fn(),
  stopQr: vi.fn(),
  requestCode: vi.fn(),
  submitCode: vi.fn(),
  submitPassword: vi.fn(),
  signOut: vi.fn(),
  passwordRequired: vi.fn(),
}));

vi.mock("../../telegram", () => ({
  telegram: {
    ...auth,
    onQr: async () => () => {},
    onAuthenticated: async () => () => {},
    onPasswordRequired: async (callback: (hint: string | null) => void) => {
      auth.passwordRequired.mockImplementation(callback);
      return () => {};
    },
    onError: async () => () => {},
  },
  errorMessage: (error: unknown) => String(error),
}));

beforeEach(() => {
  vi.clearAllMocks();
  auth.status.mockResolvedValue({ authorized: false, displayName: null });
  auth.startQr.mockResolvedValue(undefined);
  auth.stopQr.mockResolvedValue(undefined);
  auth.requestCode.mockResolvedValue({ message: "Check Telegram for your code.", length: 5 });
  auth.submitCode.mockResolvedValue({
    step: "authorized",
    status: { authorized: true, displayName: "Pedro" },
  });
  auth.signOut.mockResolvedValue(undefined);
});

afterEach(cleanup);

describe("LoginPage", () => {
  it("keeps the password challenge when QR login requires it", async () => {
    render(
      <MemoryRouter initialEntries={["/login"]}>
        <LoginPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(auth.startQr).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole("tab", { name: "Quick QR Scan" }));
    expect(auth.startQr).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("tab", { name: "Phone Number" }));
    fireEvent.click(screen.getByRole("tab", { name: "Quick QR Scan" }));
    await waitFor(() => expect(auth.startQr).toHaveBeenCalledTimes(2));
    act(() => auth.passwordRequired("Password hint"));
    await waitFor(() => expect(screen.getByLabelText("Password")).toBeInTheDocument());
    expect(screen.getByText("Hint: Password hint")).toBeInTheDocument();
    expect(auth.startQr).toHaveBeenCalledTimes(2);
  });
  it("shows QR login and switches to phone login", async () => {
    render(
      <MemoryRouter initialEntries={["/login"]}>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/connected" element={<h1>Telegram connected</h1>} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByRole("heading", { name: "Authorize Telegram" })).toBeInTheDocument();
    await waitFor(() => expect(auth.startQr).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("tab", { name: "Phone Number" }));
    expect(screen.getByLabelText("Account mobile number")).toBeInTheDocument();
  });

  it("requests a code and reaches the connected page", async () => {
    auth.submitCode.mockImplementationOnce(async () => {
      auth.status.mockResolvedValue({ authorized: true, displayName: "Pedro" });
      return { step: "authorized", status: { authorized: true, displayName: "Pedro" } };
    });
    render(
      <MemoryRouter initialEntries={["/login"]}>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/connected" element={<h1>Telegram connected</h1>} />
        </Routes>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+5511999999999" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await waitFor(() =>
      expect(screen.getByLabelText("Telegram verification code")).toBeInTheDocument(),
    );
    fireEvent.change(screen.getByLabelText("Telegram verification code"), {
      target: { value: "1234" },
    });
    expect(screen.getByRole("button", { name: "Verify code" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Telegram verification code"), {
      target: { value: "12345" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Verify code" }));
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "Telegram connected" })).toBeInTheDocument(),
    );
  });

  it("continues through the two-step password challenge", async () => {
    auth.submitCode.mockResolvedValueOnce({ step: "passwordRequired", hint: "My hint" });
    auth.submitPassword.mockImplementationOnce(async () => {
      auth.status.mockResolvedValue({ authorized: true, displayName: "Pedro" });
      return { step: "authorized", status: { authorized: true, displayName: "Pedro" } };
    });
    render(
      <MemoryRouter initialEntries={["/login"]}>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/connected" element={<h1>Telegram connected</h1>} />
        </Routes>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+5511999999999" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await waitFor(() =>
      expect(screen.getByLabelText("Telegram verification code")).toBeInTheDocument(),
    );
    fireEvent.change(screen.getByLabelText("Telegram verification code"), {
      target: { value: "12345" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Verify code" }));
    await waitFor(() => expect(screen.getByLabelText("Password")).toBeInTheDocument());
    expect(screen.getByText("Hint: My hint")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "secret" } });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(auth.submitPassword).toHaveBeenCalledWith("secret"));
  });
});
