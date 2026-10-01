import { QRCodeSVG } from "qrcode.react";
import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { QrToken } from "../../telegram";
import "./QrDisplay.css";

export function QrDisplay({ token }: { token: QrToken | null }) {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const interval = window.setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => {
      window.clearInterval(interval);
    };
  }, []);

  const seconds = token ? Math.max(0, Math.ceil(token.expiresAt - now / 1000)) : 0;
  const clock = `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;

  return (
    <div className="qr-frame">
      <div className="qr-frame__image" aria-label="Telegram login QR code">
        {token && seconds > 0 ? (
          <QRCodeSVG
            value={token.url}
            size={224}
            bgColor="var(--color-qr-background)"
            fgColor="var(--color-qr-foreground)"
            marginSize={2}
          />
        ) : (
          <div className="qr-placeholder">
            {token ? "Refreshing QR code…" : "Connecting to Telegram…"}
          </div>
        )}
      </div>
      <div className="qr-expiry">
        <RefreshCw size={15} aria-hidden="true" />
        {token && seconds > 0 ? `Expires in ${clock}` : "Requesting QR code"}
      </div>
    </div>
  );
}
