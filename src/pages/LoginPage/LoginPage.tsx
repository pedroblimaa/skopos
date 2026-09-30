import { LockKeyhole, QrCode, Send, Smartphone } from "lucide-react";
import { Button } from "../../components/Button/Button";
import { Card } from "../../components/Card/Card";
import { FormField } from "../../components/FormField/FormField";
import { QrDisplay } from "../../components/QrDisplay/QrDisplay";
import { Tabs } from "../../components/Tabs/Tabs";
import { SessionLoading } from "../../components/SessionLoading/SessionLoading";
import { VerificationCodeInput } from "../../components/VerificationCodeInput/VerificationCodeInput";
import { useTelegramLogin, type LoginMethod } from "./useTelegramLogin";
import "./LoginPage.css";

export function LoginPage() {
  const {
    method,
    step,
    qr,
    phone,
    code,
    password,
    hint,
    delivery,
    codeLength,
    error,
    isRestoringSession,
    busy,
    setPhone,
    setCode,
    setPassword,
    selectMethod,
    changePhone,
    retryQr,
    requestCode,
    submitCode,
    submitPassword,
  } = useTelegramLogin();

  if (isRestoringSession) {
    return (
      <main className="auth-page">
        <SessionLoading label="Restoring your session…" />
      </main>
    );
  }

  return (
    <main className="auth-page">
      <div className="auth-layout">
        <div className="brand">
          <QrCode size={22} aria-hidden="true" />
          <span>Skopos · Telegram Login</span>
        </div>
        <h1 className="auth-heading">Authorize Telegram</h1>
        <Card>
          <Tabs<LoginMethod>
            active={method}
            onChange={selectMethod}
            disabled={busy}
            tabs={[
              { id: "qr", label: "Quick QR Scan", icon: <QrCode size={18} /> },
              { id: "phone", label: "Phone Number", icon: <Smartphone size={18} /> },
            ]}
          />
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}
          {method === "qr" && step !== "password" && (
            <div className="qr-content">
              <QrDisplay token={qr} />
              {error && (
                <Button variant="quiet" onClick={() => void retryQr()}>
                  Try again
                </Button>
              )}
              <ol className="instructions">
                <li>Open Telegram on your phone</li>
                <li>Go to Settings → Devices → Link Desktop Device</li>
                <li>Scan this dynamic code to confirm pairing</li>
              </ol>
            </div>
          )}
          {method === "phone" && step === "phone" && (
            <form
              className="login-form"
              onSubmit={(event) => {
                void requestCode(event);
              }}
            >
              <FormField
                id="phone"
                label="Account mobile number"
                type="tel"
                autoComplete="tel"
                placeholder="+55 11 99999 9999"
                value={phone}
                onChange={(event) => {
                  setPhone(event.target.value);
                }}
                required
              />
              <Button disabled={busy} type="submit">
                <Send size={18} />
                {busy ? "Sending…" : "Send login code"}
              </Button>
              <p className="form-help">
                Include your country code. Telegram chooses how to deliver the code.
              </p>
            </form>
          )}
          {method === "phone" && step === "code" && (
            <form
              className="login-form"
              onSubmit={(event) => {
                void submitCode(event);
              }}
            >
              <p className="form-help">{delivery}</p>
              <VerificationCodeInput value={code} onChange={setCode} length={codeLength} />
              <Button
                disabled={busy || !code || (codeLength !== null && code.length !== codeLength)}
                type="submit"
              >
                {busy ? "Checking…" : "Verify code"}
              </Button>
              <Button variant="quiet" type="button" onClick={changePhone}>
                Change phone number
              </Button>
            </form>
          )}
          {step === "password" && (
            <form
              className="login-form"
              onSubmit={(event) => {
                void submitPassword(event);
              }}
            >
              <LockKeyhole className="password-icon" size={32} aria-hidden="true" />
              <p>Enter your Telegram two-step verification password.</p>
              <FormField
                id="password"
                label="Password"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(event) => {
                  setPassword(event.target.value);
                }}
                required
              />
              {hint && <p className="form-help">Hint: {hint}</p>}
              <Button disabled={busy} type="submit">
                {busy ? "Checking…" : "Continue"}
              </Button>
            </form>
          )}
        </Card>
      </div>
    </main>
  );
}
