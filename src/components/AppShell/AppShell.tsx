import { useEffect, useState } from "react";
import { Link, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Bell, ChevronDown, LogOut, Send } from "lucide-react";
import { errorMessage, telegram, type SessionStatus } from "../../telegram";
import { SessionLoading } from "../SessionLoading/SessionLoading";
import "./AppShell.css";

export function AppShell() {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [status, setStatus] = useState<SessionStatus | null>(null);
  const [error, setError] = useState("");
  const [isProfileOpen, setIsProfileOpen] = useState(false);
  const [isSigningOut, setIsSigningOut] = useState(false);
  const [signOutError, setSignOutError] = useState("");

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
        if (isActive) setError(errorMessage(reason));
      }
    }

    void loadProfile();

    return () => {
      isActive = false;
    };
  }, [navigate]);

  async function signOut() {
    setIsSigningOut(true);
    setSignOutError("");
    try {
      await telegram.signOut();
      void navigate("/login", { replace: true });
    } catch (reason) {
      setSignOutError(errorMessage(reason));
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
          aria-label="Telegram profile"
          aria-expanded={isProfileOpen}
          aria-controls="telegram-profile-menu"
          disabled={!status}
        >
          <span className="app-profile-icon">
            <Send size={16} aria-hidden="true" />
          </span>
          <span className="app-profile-details">
            <span className="app-profile-label">Telegram</span>
            <strong>{status?.displayName ?? "Telegram user"}</strong>
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
          <button
            className="app-sign-out"
            type="button"
            disabled={isSigningOut}
            onClick={() => void signOut()}
          >
            <LogOut size={16} aria-hidden="true" />
            {isSigningOut ? "Signing out…" : "Sign out"}
          </button>
          {signOutError && (
            <p className="app-sign-out-error" role="alert">
              {signOutError}
            </p>
          )}
        </div>
      </header>
      {error && (
        <p className="error app-session-error" role="alert">
          {error}
        </p>
      )}
      {status && (
        <div className="app-page-transition" key={pathname}>
          <Outlet />
        </div>
      )}
      {!status && !error && <SessionLoading label="Loading your Telegram profile…" />}
    </div>
  );
}
