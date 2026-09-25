import { useEffect, useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { errorMessage, telegram, type LoginResult, type QrToken } from "../../telegram";

export type Method = "qr" | "phone";
type Step = "phone" | "code" | "password";

export function useTelegramLogin() {
  const navigate = useNavigate();
  const [method, setMethod] = useState<Method>("qr");
  const [step, setStep] = useState<Step>("phone");
  const [qr, setQr] = useState<QrToken | null>(null);
  const [phone, setPhone] = useState("");
  const [code, setCode] = useState("");
  const [password, setPassword] = useState("");
  const [hint, setHint] = useState<string | null>(null);
  const [delivery, setDelivery] = useState("");
  const [codeLength, setCodeLength] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [ready, setReady] = useState(false);
  const [subscribed, setSubscribed] = useState(false);

  useEffect(() => {
    let active = true;
    async function checkSession() {
      try {
        const status = await telegram.status();
        if (!active) return;
        if (status.authorized) navigate("/connected", { replace: true });
        else setReady(true);
      } catch (reason) {
        if (active) {
          setError(errorMessage(reason));
          setReady(true);
        }
      }
    }
    void checkSession();
    return () => {
      active = false;
    };
  }, [navigate]);

  useEffect(() => {
    if (!ready) return;
    let active = true;
    const unlisten: Array<() => void> = [];
    async function subscribe() {
      try {
        unlisten.push(
          await telegram.onQr((token) => {
            if (active) {
              setQr(token);
              setError("");
            }
          }),
        );
        unlisten.push(
          await telegram.onAuthenticated((status) => {
            if (active && status.authorized) navigate("/connected", { replace: true });
          }),
        );
        unlisten.push(
          await telegram.onPasswordRequired((value) => {
            if (active) {
              setHint(value);
              setStep("password");
            }
          }),
        );
        unlisten.push(
          await telegram.onError((message) => {
            if (active) setError(message);
          }),
        );
        if (active) setSubscribed(true);
        else unlisten.forEach((stop) => stop());
      } catch (reason) {
        unlisten.forEach((stop) => stop());
        if (active) setError(errorMessage(reason));
      }
    }
    void subscribe();
    return () => {
      active = false;
      unlisten.forEach((stop) => stop());
    };
  }, [ready, navigate]);

  useEffect(() => {
    if (!subscribed || method !== "qr" || step === "password") return;
    let active = true;
    async function startQr() {
      try {
        await telegram.startQr();
      } catch (reason) {
        if (active) setError(errorMessage(reason));
      }
    }
    void startQr();
    return () => {
      active = false;
      void telegram.stopQr().catch(() => {});
    };
  }, [subscribed, method, step]);

  function handleResult(result: LoginResult) {
    if (result.step === "authorized") navigate("/connected", { replace: true });
    else {
      setHint(result.hint);
      setStep("password");
    }
  }

  function selectMethod(next: Method) {
    if (next === method) return;
    setMethod(next);
    setError("");
    setQr(null);
    setStep("phone");
    setCode("");
    setPassword("");
  }

  function changePhone() {
    setCode("");
    setStep("phone");
    setError("");
  }

  async function retryQr() {
    setError("");
    setQr(null);
    try {
      await telegram.startQr();
    } catch (reason) {
      setError(errorMessage(reason));
    }
  }

  async function requestCode(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      const result = await telegram.requestCode(phone.replace(/[\s()-]/g, ""));
      setCode("");
      setDelivery(result.message);
      setCodeLength(result.length);
      setStep("code");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function submitCode(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      handleResult(await telegram.submitCode(code));
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function submitPassword(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      handleResult(await telegram.submitPassword(password));
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
      setPassword("");
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
  };
}
