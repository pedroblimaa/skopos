import { useLanguage } from "../../i18n/useLanguage";
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
  const { t, message } = useLanguage();
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
        <SessionLoading label={t("restoringSession")} />
      </main>
    );
  }

  return (
    <main className="auth-page">
      <div className="auth-layout">
        <div className="brand">
          <QrCode size={22} aria-hidden="true" />
          <span>Skopos · {t("telegramLogin")}</span>
        </div>
        <h1 className="auth-heading">{t("authorizeTelegram")}</h1>
        <Card>
          <Tabs<LoginMethod>
            label={t("loginMethod")}
            active={method}
            onChange={selectMethod}
            disabled={busy}
            tabs={[
              { id: "qr", label: t("quickQrScan"), icon: <QrCode size={18} /> },
              { id: "phone", label: t("phoneNumber"), icon: <Smartphone size={18} /> },
            ]}
          />
          {error !== null && (
            <p role="alert" className="error">
              {message(error)}
            </p>
          )}
          {method === "qr" && step !== "password" && (
            <div className="qr-content">
              <QrDisplay token={qr} />
              {error !== null && (
                <Button variant="quiet" onClick={() => void retryQr()}>
                  {t("tryAgain")}
                </Button>
              )}
              <ol className="instructions">
                <li>{t("openTelegram")}</li>
                <li>{t("linkDevice")}</li>
                <li>{t("scanCode")}</li>
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
                label={t("accountMobileNumber")}
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
                {busy ? t("sending") : t("sendLoginCode")}
              </Button>
              <p className="form-help">{t("phoneHelp")}</p>
            </form>
          )}
          {method === "phone" && step === "code" && (
            <form
              className="login-form"
              onSubmit={(event) => {
                void submitCode(event);
              }}
            >
              {delivery !== null && <p className="form-help">{message(delivery)}</p>}
              <VerificationCodeInput value={code} onChange={setCode} length={codeLength} />
              <Button
                disabled={busy || !code || (codeLength !== null && code.length !== codeLength)}
                type="submit"
              >
                {busy ? t("checking") : t("verifyCode")}
              </Button>
              <Button variant="quiet" type="button" onClick={changePhone}>
                {t("changePhoneNumber")}
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
              <p>{t("passwordInstructions")}</p>
              <FormField
                id="password"
                label={t("password")}
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(event) => {
                  setPassword(event.target.value);
                }}
                required
              />
              {hint && <p className="form-help">{t("passwordHint", { hint })}</p>}
              <Button disabled={busy} type="submit">
                {busy ? t("checking") : t("continue")}
              </Button>
            </form>
          )}
        </Card>
      </div>
    </main>
  );
}
