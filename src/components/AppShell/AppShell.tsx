import { useEffect, useState } from "react";
import { Link, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Bell, ChevronDown, LogOut, Send } from "lucide-react";
import { telegram, type SessionStatus } from "../../telegram";
import { SessionLoading } from "../SessionLoading/SessionLoading";
import { InfoTooltip } from "../InfoTooltip/InfoTooltip";
import { appError, type AppMessage } from "../../app-message";
import { useLanguage } from "../../i18n/useLanguage";
import "./AppShell.css";

export function AppShell() {
  const { t, message, language, setLanguage } = useLanguage();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [status, setStatus] = useState<SessionStatus | null>(null);
  const [error, setError] = useState<AppMessage | null>(null);
  const [isProfileOpen, setIsProfileOpen] = useState(false);
  const [isSigningOut, setIsSigningOut] = useState(false);
  const [signOutError, setSignOutError] = useState<AppMessage | null>(null);

  useEffect(() => {
    let isActive = true;

    async function loadProfile() {
      try {
        const profile = await telegram.status();
        if (!isActive) return;

        if (!profile.authorized) {
          void navigate("/login", { replace: true });
          return;
        }

        setStatus(profile);
      } catch (reason) {
        if (isActive) setError(appError(reason));
      }
    }

    void loadProfile();

    return () => {
      isActive = false;
    };
  }, [navigate]);

  async function signOut() {
    setIsSigningOut(true);
    setSignOutError(null);

    try {
      await telegram.signOut();
      void navigate("/login", { replace: true });
    } catch (reason) {
      setSignOutError(appError(reason));
      setIsSigningOut(false);
    }
  }

  return (
    <div className="app-shell">
      <header className="app-topbar">
        <Link className="app-brand" to="/connected">
          <Bell size={21} aria-hidden="true" /> Skopos
        </Link>
        <button
          className="app-profile"
          type="button"
          popoverTarget="telegram-profile-menu"
          aria-label={t("telegramProfile")}
          aria-expanded={isProfileOpen}
          aria-controls="telegram-profile-menu"
          disabled={!status}
        >
          <span className="app-profile-icon">
            <Send size={16} aria-hidden="true" />
          </span>
          <span className="app-profile-details">
            <span className="app-profile-label">Telegram</span>
            <strong>{status?.displayName ?? t("telegramUser")}</strong>
          </span>
          <ChevronDown size={14} aria-hidden="true" />
        </button>
        <div
          className="app-profile-menu"
          id="telegram-profile-menu"
          popover="auto"
          onToggle={(event) => {
            setIsProfileOpen(event.newState === "open");
          }}
        >
          <div className="app-language">
            <span id="app-language-label">{t("language")}</span>
            <div
              className="app-language-segments"
              role="group"
              aria-labelledby="app-language-label"
            >
              <InfoTooltip
                label="Português (Brasil)"
                trigger="PT-BR"
                buttonProps={{
                  "className": "app-language-segment",
                  "lang": "pt-BR",
                  "aria-pressed": language === "pt-BR",
                  "onClick": () => {
                    setLanguage("pt-BR");
                  },
                }}
              >
                Português (Brasil)
              </InfoTooltip>
              <InfoTooltip
                label="English"
                trigger="EN"
                buttonProps={{
                  "className": "app-language-segment",
                  "lang": "en",
                  "aria-pressed": language === "en",
                  "onClick": () => {
                    setLanguage("en");
                  },
                }}
              >
                English
              </InfoTooltip>
            </div>
          </div>
          <hr className="app-profile-separator" />
          <button
            className="app-sign-out"
            type="button"
            disabled={isSigningOut}
            onClick={() => void signOut()}
          >
            <LogOut size={16} aria-hidden="true" />
            {isSigningOut ? t("signingOut") : t("signOut")}
          </button>
          {signOutError !== null && (
            <p className="app-sign-out-error" role="alert">
              {message(signOutError)}
            </p>
          )}
        </div>
      </header>
      {error !== null && (
        <p className="error app-session-error" role="alert">
          {message(error)}
        </p>
      )}
      {status !== null && (
        <div className="app-page-transition" key={pathname}>
          <Outlet />
        </div>
      )}
      {!status && !error && <SessionLoading label={t("loadingProfile")} />}
    </div>
  );
}
