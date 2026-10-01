import { useEffect, useState, type SyntheticEvent } from "react";
import { useNavigate } from "react-router-dom";
import { isCodeSubmissionError, telegram, type LoginResult, type QrToken } from "../../telegram";
import { appError, type AppMessage } from "../../app-message";

export type LoginMethod = "qr" | "phone";
type Step = "phone" | "code" | "password";

export function useTelegramLogin() {
  const navigate = useNavigate();
  const [method, setMethod] = useState<LoginMethod>("qr");
  const [step, setStep] = useState<Step>("phone");
  const [qr, setQr] = useState<QrToken | null>(null);
  const [phone, setPhone] = useState("");
  const [code, setCode] = useState("");
  const [password, setPassword] = useState("");
  const [hint, setHint] = useState<string | null>(null);
  const [delivery, setDelivery] = useState<AppMessage | null>(null);
  const [codeLength, setCodeLength] = useState<number | null>(null);
  const [error, setError] = useState<AppMessage | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [isSessionChecked, setIsSessionChecked] = useState(false);
  const [isSubscribed, setIsSubscribed] = useState(false);

  useEffect(() => {
    let active = true;

    async function checkSession() {
      try {
        const status = await telegram.status();

        if (!active) return;

        if (status.authorized) void navigate("/connected", { replace: true });
        else setIsSessionChecked(true);
      } catch (reason) {
        if (active) {
          setError(appError(reason));
          setIsSessionChecked(true);
        }
      }
    }

    void checkSession();

    return () => {
      active = false;
    };
  }, [navigate]);

  useEffect(() => {
    if (!isSessionChecked) return;

    let active = true;
    const unlisten: Array<() => void> = [];

    function cleanup() {
      active = false;

      unlisten.splice(0).forEach((stop) => {
        stop();
      });
    }

    async function addListener<T>(
      listen: (callback: (value: T) => void) => Promise<() => void>,
      callback: (value: T) => void,
    ) {
      const stop = await listen((value) => {
        if (active) callback(value);
      });

      if (!active) {
        stop();
        return false;
      }

      unlisten.push(stop);
      return true;
    }

    async function subscribe() {
      try {
        const listeners = [
          () => addListener(telegram.onQr, setQr),
          () =>
            addListener(telegram.onAuthenticated, (status) => {
              if (status.authorized) void navigate("/connected", { replace: true });
            }),
          () =>
            addListener(telegram.onPasswordRequired, (value) => {
              setHint(value);
              setStep("password");
            }),
          () => addListener(telegram.onError, setError),
        ];

        for (const register of listeners) {
          if (!(await register())) return;
        }

        if (active) setIsSubscribed(true);
      } catch (reason) {
        if (active) setError(appError(reason));

        cleanup();
      }
    }

    void subscribe();

    return cleanup;
  }, [isSessionChecked, navigate]);

  useEffect(() => {
    if (!isSubscribed || method !== "qr" || step === "password") return;

    let active = true;

    async function startQr() {
      try {
        await telegram.startQr();
      } catch (reason) {
        if (active) setError(appError(reason));
      }
    }

    void startQr();

    return () => {
      active = false;
      void telegram.stopQr().catch(() => {});
    };
  }, [isSubscribed, method, step]);

  function selectMethod(next: LoginMethod) {
    if (next === method) return;

    setMethod(next);
    setError(null);
    setQr(null);
    setStep("phone");
    setCode("");
    setPassword("");
  }

  function changePhone() {
    setCode("");
    setStep("phone");
    setError(null);
  }

  async function retryQr() {
    setError(null);
    setQr(null);

    try {
      await telegram.startQr();
    } catch (reason) {
      setError(appError(reason));
    }
  }

  async function requestCode(event: SyntheticEvent<HTMLFormElement>) {
    await runSubmission(event, async () => {
      const result = await telegram.requestCode(phone.replace(/[\s()-]/g, ""));

      if (result.step === "authorized") {
        void navigate("/connected", { replace: true });
        return;
      }

      setCode("");
      setDelivery(result.message);
      setCodeLength(result.length);
      setStep("code");
    });
  }

  async function submitCode(event: SyntheticEvent<HTMLFormElement>) {
    await runSubmission(
      event,
      async () => {
        handleResult(await telegram.submitCode(code));
      },
      (reason) => {
        if (!isCodeSubmissionError(reason) || !reason.canRetryCode) {
          setCode("");
          setStep("phone");
        }
      },
    );
  }

  async function submitPassword(event: SyntheticEvent<HTMLFormElement>) {
    await runSubmission(
      event,
      async () => {
        handleResult(await telegram.submitPassword(password));
      },
      (reason) => {
        if (appError(reason).code !== "incorrectPassword") {
          setHint(null);
          setStep("phone");
        }
      },
    );
    setPassword("");
  }

  function handleResult(result: LoginResult) {
    if (result.step === "authorized") void navigate("/connected", { replace: true });
    else {
      setHint(result.hint);
      setStep("password");
    }
  }

  async function runSubmission(
    event: SyntheticEvent<HTMLFormElement>,
    action: () => Promise<void>,
    onError?: (reason: unknown) => void,
  ) {
    event.preventDefault();

    setIsBusy(true);
    setError(null);

    try {
      await action();
    } catch (reason) {
      onError?.(reason);
      setError(appError(reason));
    } finally {
      setIsBusy(false);
    }
  }

  return {
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
    isRestoringSession: !isSessionChecked,
    busy: isBusy,
    setPhone,
    setCode,
    setPassword,
    selectMethod,
    changePhone,
    retryQr,
    requestCode,
    submitCode,
    submitPassword,
  };
}
