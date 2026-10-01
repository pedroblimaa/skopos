import type { AppMessage } from "../../app-message";
import { render } from "../../test-setup";
import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { LoginPage } from "./LoginPage";
import type { QrToken, SessionStatus } from "../../telegram";

const auth = vi.hoisted(() => ({
  status: vi.fn(),
  startQr: vi.fn(),
  stopQr: vi.fn(),
  requestCode: vi.fn(),
  submitCode: vi.fn(),
  submitPassword: vi.fn(),
  signOut: vi.fn(),
  passwordRequired: vi.fn<(hint: string | null) => void>(),
  onQr: vi.fn<(callback: (token: QrToken) => void) => Promise<() => void>>(),
  onAuthenticated: vi.fn<(callback: (status: SessionStatus) => void) => Promise<() => void>>(),
  onPasswordRequired: vi.fn<(callback: (hint: string | null) => void) => Promise<() => void>>(),
  onError: vi.fn<(callback: (message: AppMessage) => void) => Promise<() => void>>(),
}));

vi.mock("../../telegram", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../telegram")>()),
  telegram: auth,
}));

beforeEach(() => {
  vi.resetAllMocks();

  auth.status.mockResolvedValue({ authorized: false, displayName: null });
  auth.startQr.mockResolvedValue(undefined);
  auth.stopQr.mockResolvedValue(undefined);
  auth.requestCode.mockResolvedValue({
    step: "codeSent",
    message: { code: "deliveryApp" },
    length: 5,
  });
  auth.submitCode.mockResolvedValue({
    step: "authorized",
    status: { authorized: true, displayName: "Pedro" },
  });
  auth.signOut.mockResolvedValue(undefined);

  auth.onQr.mockResolvedValue(() => {});
  auth.onAuthenticated.mockResolvedValue(() => {});
  auth.onPasswordRequired.mockImplementation((callback) => {
    auth.passwordRequired.mockImplementation(callback);
    return Promise.resolve(() => {});
  });
  auth.onError.mockResolvedValue(() => {});
});

afterEach(cleanup);

describe("LoginPage", () => {
  it("shows session restoration instead of login while checking a saved session", async () => {
    let resolveStatus!: (status: SessionStatus) => void;
    auth.status.mockReturnValueOnce(
      new Promise<SessionStatus>((resolve) => {
        resolveStatus = resolve;
      }),
    );

    renderLogin();

    expect(screen.getByRole("status")).toHaveTextContent("Restoring your session");
    expect(screen.queryByRole("heading", { name: "Authorize Telegram" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Telegram login QR code")).not.toBeInTheDocument();
    expect(auth.startQr).not.toHaveBeenCalled();

    await act(async () => {
      resolveStatus({ authorized: true, displayName: "Pedro" });
      await Promise.resolve();
    });

    expect(screen.getByRole("heading", { name: "Telegram connected" })).toBeInTheDocument();
    expect(auth.startQr).not.toHaveBeenCalled();
  });

  it("handles QR, error, and authorization events after subscribing", async () => {
    renderLogin();
    await waitFor(() => {
      expect(auth.startQr).toHaveBeenCalled();
    });

    act(() => {
      auth.onQr.mock.calls[0][0]({ url: "tg://login?token=test", expiresAt: 123 });
      auth.onError.mock.calls[0][0]({ code: "authNetwork" });
      auth.onAuthenticated.mock.calls[0][0]({ authorized: false, displayName: null });
    });
    expect(screen.getByRole("alert")).toHaveTextContent("Could not reach Telegram");

    act(() => {
      auth.onAuthenticated.mock.calls[0][0]({ authorized: true, displayName: "Pedro" });
    });
    expect(screen.getByRole("heading", { name: "Telegram connected" })).toBeInTheDocument();
  });

  it("cleans up a partial subscription once when registration fails", async () => {
    const stopQr = vi.fn();
    auth.onQr.mockResolvedValueOnce(stopQr);
    auth.onAuthenticated.mockRejectedValueOnce({ code: "authNetwork" });

    const { unmount } = renderLogin();
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("Could not reach Telegram"),
    );
    expect(stopQr).toHaveBeenCalledTimes(1);
    expect(auth.onPasswordRequired).not.toHaveBeenCalled();
    expect(auth.startQr).not.toHaveBeenCalled();

    unmount();
    expect(stopQr).toHaveBeenCalledTimes(1);
  });

  const listenerNames = ["onQr", "onAuthenticated", "onPasswordRequired", "onError"] as const;

  it.each(
    listenerNames.flatMap((name) => [
      { name, fails: false },
      { name, fails: true },
    ]),
  )(
    "cleans up pending $name listeners after unmount (failure: $fails)",
    async ({ name, fails }) => {
      const stops = [vi.fn(), vi.fn(), vi.fn(), vi.fn()];
      let resolve!: (stop: () => void) => void;
      let reject!: (reason: Error) => void;
      const registration = new Promise<() => void>((res, rej) => {
        resolve = res;
        reject = rej;
      });

      const pendingIndex = listenerNames.indexOf(name);

      for (const [index, listener] of listenerNames.entries()) {
        if (listener === name) auth[listener].mockReturnValueOnce(registration);
        else auth[listener].mockResolvedValueOnce(stops[index]);
      }

      const { unmount } = renderLogin();
      await waitFor(() => {
        expect(auth[name]).toHaveBeenCalled();
      });

      unmount();
      for (const stop of stops.slice(0, pendingIndex)) {
        expect(stop).toHaveBeenCalledTimes(1);
      }

      await act(async () => {
        auth.onQr.mock.calls[0][0]({ url: "tg://login?token=late", expiresAt: 123 });
        if (fails) reject(new Error("Late registration failure"));
        else resolve(stops[pendingIndex]);
        await registration.catch(() => {});
      });

      for (const stop of stops.slice(0, pendingIndex + (fails ? 0 : 1))) {
        expect(stop).toHaveBeenCalledTimes(1);
      }
      for (const listener of listenerNames.slice(pendingIndex + 1)) {
        expect(auth[listener]).not.toHaveBeenCalled();
      }
      expect(auth.startQr).not.toHaveBeenCalled();
    },
  );

  it("shows a QR error and retries", async () => {
    auth.startQr.mockRejectedValueOnce({ code: "authNetwork" });

    renderLogin();
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("Could not reach Telegram"),
    );

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => {
      expect(auth.startQr).toHaveBeenCalledTimes(2);
    });
  });

  it("shows a session lookup failure and still offers login", async () => {
    auth.status.mockRejectedValueOnce({ code: "authStorage" });

    renderLogin();
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(
        "Could not access the local Telegram session",
      ),
    );
    await waitFor(() => {
      expect(auth.startQr).toHaveBeenCalled();
    });
  });

  it("keeps the phone form after code request failure", async () => {
    auth.requestCode.mockRejectedValueOnce({ code: "floodWait" });

    renderLogin();

    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+55 11 99999 9999" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("Telegram is limiting login attempts"),
    );
    expect(screen.getByLabelText("Account mobile number")).toBeInTheDocument();
    expect(auth.requestCode).toHaveBeenCalledWith("+5511999999999");
  });

  it("keeps the password challenge when QR login requires it", async () => {
    renderLogin();
    await waitFor(() => {
      expect(auth.startQr).toHaveBeenCalledTimes(1);
    });

    fireEvent.click(screen.getByRole("tab", { name: "Quick QR Scan" }));
    expect(auth.startQr).toHaveBeenCalledTimes(1);

    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    fireEvent.click(screen.getByRole("tab", { name: "Quick QR Scan" }));
    await waitFor(() => {
      expect(auth.startQr).toHaveBeenCalledTimes(2);
    });

    act(() => {
      auth.passwordRequired("Password hint");
    });
    await waitFor(() => expect(screen.getByLabelText("Password")).toBeInTheDocument());
    expect(screen.getByText("Hint: Password hint")).toBeInTheDocument();
    expect(auth.startQr).toHaveBeenCalledTimes(2);
  });

  it("shows QR login and switches to phone login", async () => {
    renderLogin();

    expect(await screen.findByRole("heading", { name: "Authorize Telegram" })).toBeInTheDocument();
    await waitFor(() => {
      expect(auth.startQr).toHaveBeenCalled();
    });

    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    expect(screen.getByLabelText("Account mobile number")).toBeInTheDocument();
  });

  it("requests a code and reaches the connected page", async () => {
    auth.submitCode.mockImplementationOnce(() => {
      auth.status.mockResolvedValue({ authorized: true, displayName: "Pedro" });
      return Promise.resolve({
        step: "authorized",
        status: { authorized: true, displayName: "Pedro" },
      });
    });

    renderLogin();

    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
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

  it("opens the connected page when the phone request authorizes immediately", async () => {
    auth.requestCode.mockResolvedValueOnce({
      step: "authorized",
      status: { authorized: true, displayName: "Pedro" },
    });

    renderLogin();

    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+5511999999999" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "Telegram connected" })).toBeInTheDocument(),
    );
    expect(auth.submitCode).not.toHaveBeenCalled();
  });

  it("returns to phone entry when a verification code expires", async () => {
    auth.submitCode.mockRejectedValueOnce({
      message: { code: "codeExpired" },
      canRetryCode: false,
    });

    renderLogin();
    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+5511999999999" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await screen.findByLabelText("Telegram verification code");

    fireEvent.change(screen.getByLabelText("Telegram verification code"), {
      target: { value: "12345" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Verify code" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Account mobile number")).toBeInTheDocument();
    });
    expect(screen.getByRole("alert")).toHaveTextContent("This code expired");
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await waitFor(() => {
      expect(auth.requestCode).toHaveBeenCalledTimes(2);
    });
  });

  it("returns to phone entry when session verification fails after code submission", async () => {
    auth.submitCode.mockRejectedValueOnce({
      message: { code: "authNetwork" },
      canRetryCode: false,
    });

    renderLogin();
    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+5511999999999" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await screen.findByLabelText("Telegram verification code");

    fireEvent.change(screen.getByLabelText("Telegram verification code"), {
      target: { value: "12345" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Verify code" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Account mobile number")).toBeInTheDocument();
    });
    expect(screen.getByRole("alert")).toHaveTextContent("Could not reach Telegram");
  });

  it("keeps code entry when the submitted code is invalid", async () => {
    auth.submitCode.mockRejectedValueOnce({
      message: { code: "invalidCode" },
      canRetryCode: true,
    });

    renderLogin();
    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+5511999999999" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await screen.findByLabelText("Telegram verification code");

    fireEvent.change(screen.getByLabelText("Telegram verification code"), {
      target: { value: "00000" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Verify code" }));

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("invalid");
    });
    expect(screen.getByLabelText("Telegram verification code")).toBeInTheDocument();
  });

  it("restarts login after a password request fails", async () => {
    auth.submitCode.mockResolvedValueOnce({ step: "passwordRequired", hint: null });
    auth.submitPassword.mockRejectedValueOnce({ code: "authNetwork" });

    renderLogin();
    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
    fireEvent.change(screen.getByLabelText("Account mobile number"), {
      target: { value: "+5511999999999" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await screen.findByLabelText("Telegram verification code");

    fireEvent.change(screen.getByLabelText("Telegram verification code"), {
      target: { value: "12345" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Verify code" }));
    await screen.findByLabelText("Password");

    fireEvent.change(screen.getByLabelText("Password"), { target: { value: "secret" } });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    await waitFor(() => {
      expect(screen.getByLabelText("Account mobile number")).toBeInTheDocument();
    });
    expect(screen.getByRole("alert")).toHaveTextContent("Could not reach Telegram");
    fireEvent.click(screen.getByRole("button", { name: "Send login code" }));
    await waitFor(() => {
      expect(auth.requestCode).toHaveBeenCalledTimes(2);
    });
  });

  it("continues through the two-step password challenge", async () => {
    auth.submitCode.mockResolvedValueOnce({ step: "passwordRequired", hint: "My hint" });
    auth.submitPassword.mockImplementationOnce(() => {
      auth.status.mockResolvedValue({ authorized: true, displayName: "Pedro" });
      return Promise.resolve({
        step: "authorized",
        status: { authorized: true, displayName: "Pedro" },
      });
    });

    renderLogin();

    fireEvent.click(await screen.findByRole("tab", { name: "Phone Number" }));
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
    await waitFor(() => {
      expect(auth.submitPassword).toHaveBeenCalledWith("secret");
    });
  });
});

function renderLogin() {
  return render(
    <MemoryRouter initialEntries={["/login"]}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/connected" element={<h1>Telegram connected</h1>} />
      </Routes>
    </MemoryRouter>,
  );
}
