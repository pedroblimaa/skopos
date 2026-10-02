import { useEffect, useState } from "react";
import { Navigate, NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { ChevronDown, LogOut, Send } from "lucide-react";
import { Titlebar } from "../Titlebar/Titlebar";
import { telegram, type SessionStatus } from "../../telegram";
import { SessionLoading } from "../SessionLoading/SessionLoading";
import { InfoTooltip } from "../InfoTooltip/InfoTooltip";
import { appError, type AppMessage } from "../../app-message";
import { useLanguage } from "../../i18n/useLanguage";
import { ChatsContext } from "../../pages/ChatsPage/chats-context";
import { useChatCache } from "../../pages/ChatsPage/useChatCache";
import "./AppShell.css";

export function AppShell() {
  const chatCache = useChatCache();
  const { t, message, language, setLanguage } = useLanguage();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [status, setStatus] = useState<SessionStatus | null>(null);
  const [profilePhoto, setProfilePhoto] = useState<string | null>(null);
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

        setStatus(profile);
        if (!profile.authorized) return;

        await loadPhoto();
      } catch (reason) {
        if (isActive) setError(appError(reason));
      }
    }

    async function loadPhoto() {
      try {
        const photo = await telegram.getProfilePhoto();
        if (isActive) setProfilePhoto(photo);
      } catch {
        // The avatar is optional; keep the Telegram icon if the download fails.
        if (isActive) setProfilePhoto(null);
      }
    }

    void loadProfile();

    return () => {
      isActive = false;
    };
  }, []);

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

  if (status && !status.authorized) return <Navigate to="/login" replace />;

  return (
    <div className="app-shell">
      <Titlebar homePath="/connected">
        <button
          className="app-profile"
          type="button"
          popoverTarget="telegram-profile-menu"
          aria-label={t("telegramProfile")}
          aria-expanded={isProfileOpen}
          aria-controls="telegram-profile-menu"
          disabled={!status}
        >
          <ChevronDown size={14} aria-hidden="true" />
          <span className="app-profile-details">
            <span className="app-profile-label">Telegram</span>
            <strong>{status?.displayName ?? t("telegramUser")}</strong>
          </span>
          <span className="app-profile-icon">
            {profilePhoto ? (
              <img
                src={profilePhoto}
                alt=""
                onError={() => {
                  setProfilePhoto(null);
                }}
              />
            ) : (
              <Send size={16} aria-hidden="true" />
            )}
          </span>
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
              data-language={language}
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
      </Titlebar>
      {status !== null && (
        <nav className="app-navigation" aria-label={t("appNavigation")}>
          <NavLink
            to="/connected"
            className={() =>
              `app-navigation-link${pathname !== "/chats" ? " app-navigation-link--active" : ""}`
            }
          >
            {t("products")}
          </NavLink>
          <NavLink
            to="/chats"
            className={({ isActive }) =>
              `app-navigation-link${isActive ? " app-navigation-link--active" : ""}`
            }
          >
            {t("chats")}
          </NavLink>
        </nav>
      )}
      {error !== null && (
        <p className="error app-session-error" role="alert">
          {message(error)}
        </p>
      )}
      {status !== null && (
        <ChatsContext.Provider value={chatCache}>
          <div className="app-page-transition" key={pathname}>
            <Outlet />
          </div>
        </ChatsContext.Provider>
      )}
      {!status && !error && <SessionLoading label={t("loadingProfile")} />}
    </div>
  );
}
