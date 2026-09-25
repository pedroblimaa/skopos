import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { CheckCircle2 } from "lucide-react";
import { Button } from "../../components/Button/Button";
import { Card } from "../../components/Card/Card";
import { errorMessage, telegram, type SessionStatus } from "../../telegram";
import "./ConnectedPage.css";

export function ConnectedPage() {
  const navigate = useNavigate();
  const [status, setStatus] = useState<SessionStatus | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    async function checkSession() {
      try {
        const result = await telegram.status();
        if (!active) return;
        if (!result.authorized) navigate("/login", { replace: true });
        else setStatus(result);
      } catch (reason) {
        if (active) setError(errorMessage(reason));
      }
    }
    void checkSession();
    return () => {
      active = false;
    };
  }, [navigate]);

  async function disconnect() {
    setBusy(true);
    setError("");
    try {
      await telegram.signOut();
      navigate("/login", { replace: true });
    } catch (reason) {
      setError(errorMessage(reason));
      setBusy(false);
    }
  }

  return (
    <main className="auth-page connected-page">
      <div className="auth-layout">
        <h1 className="auth-heading">Telegram connected</h1>
        <Card>
          <CheckCircle2 className="connected-icon" size={48} aria-hidden="true" />
          <p>
            {status
              ? `Signed in as ${status.displayName ?? "Telegram user"}`
              : "Checking Telegram session…"}
          </p>
          <p className="form-help">Your Telegram session is saved on this device.</p>
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}
          <Button disabled={busy || !status} onClick={disconnect}>
            {busy ? "Disconnecting…" : "Disconnect Telegram"}
          </Button>
        </Card>
      </div>
    </main>
  );
}
