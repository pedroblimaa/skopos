import { useState } from "react";
import { MessagesSquare, RefreshCw } from "lucide-react";
import { Button } from "../../components/Button/Button";
import { SearchField } from "../../components/SearchField/SearchField";
import { SessionLoading } from "../../components/SessionLoading/SessionLoading";
import { useLanguage } from "../../i18n/useLanguage";
import { useChats } from "./useChats";
import "./ChatsPage.css";

export function ChatsPage() {
  const { t, message, language } = useLanguage();
  const state = useChats();
  const [query, setQuery] = useState("");
  const search = query.trim().toLocaleLowerCase(language);
  const chats = state.chats
    .filter((chat) =>
      `${chat.title} ${chat.username ? `@${chat.username}` : ""}`
        .toLocaleLowerCase(language)
        .includes(search),
    )
    .sort((a, b) => a.title.localeCompare(b.title, language) || a.id.localeCompare(b.id));
  const isBusy = state.isLoading || state.isSaving;

  return (
    <main className="chats-page">
      <section aria-labelledby="chats-heading">
        <div className="chats-heading">
          <div>
            <h1 id="chats-heading">{t("chats")}</h1>
            <p>{t("chatsHelp")}</p>
          </div>
        </div>
        {state.error !== null && (
          <div className="error" role="alert">
            <p>{message(state.error)}</p>
            <Button variant="quiet" disabled={isBusy} onClick={() => void state.retry()}>
              {t("tryAgain")}
            </Button>
          </div>
        )}
        {state.isLoading && !state.isLoaded && <SessionLoading label={t("loadingChats")} />}
        {!state.isLoaded && !state.isLoading && state.error !== null && (
          <Button
            iconOnly
            variant="quiet"
            aria-label={t("refreshChats")}
            title={t("refreshChats")}
            onClick={() => void state.refresh()}
          >
            <RefreshCw size={18} aria-hidden="true" />
          </Button>
        )}
        {state.isLoaded && (
          <>
            <div className="chats-toolbar">
              <SearchField
                id="chat-search"
                label={t("searchChats")}
                value={query}
                onChange={(event) => {
                  setQuery(event.target.value);
                }}
              />
              <Button
                iconOnly
                variant="quiet"
                disabled={isBusy}
                aria-label={t("refreshChats")}
                title={t("refreshChats")}
                onClick={() => void state.refresh()}
              >
                <RefreshCw
                  className={state.isLoading ? "chats-refreshing" : undefined}
                  size={18}
                  aria-hidden="true"
                />
              </Button>
            </div>
            {state.chats.length === 0 && (
              <div className="chats-empty">
                <MessagesSquare size={28} aria-hidden="true" />
                <h2>{t("emptyChatsTitle")}</h2>
                <p>{t("emptyChatsHelp")}</p>
              </div>
            )}
            {state.chats.length > 0 && chats.length === 0 && (
              <p className="chats-empty">{t("noMatchingChats")}</p>
            )}
            {chats.length > 0 && (
              <ul className="chats-list" aria-label={t("promotionChats")}>
                {chats.map((chat) => (
                  <li key={chat.id}>
                    <label className="chats-row interactive-row">
                      <input
                        type="checkbox"
                        checked={state.selected.has(chat.id)}
                        disabled={isBusy || (!chat.available && !state.selected.has(chat.id))}
                        onChange={() => {
                          state.toggle(chat.id);
                        }}
                      />
                      <span className="chats-avatar" aria-hidden="true">
                        {state.photos[chat.id] ? (
                          <img src={state.photos[chat.id]} alt="" />
                        ) : (
                          chat.title
                            .replace(/[^\p{L}\p{N} ]/gu, "")
                            .trim()
                            .split(/\s+/u)
                            .slice(0, 2)
                            .map((word) => word[0])
                            .join("")
                            .toLocaleUpperCase(language)
                        )}
                      </span>
                      <span className="chats-details">
                        <strong>{chat.title}</strong>
                        <span>
                          {t(chat.kind === "group" ? "chatGroup" : "chatChannel")}
                          {chat.username && ` · @${chat.username}`}
                          {!chat.available && ` · ${t("chatUnavailable")}`}
                        </span>
                      </span>
                    </label>
                  </li>
                ))}
              </ul>
            )}
            <div className="chats-footer">
              <div className="chats-actions">
                <Button
                  variant="quiet"
                  disabled={isBusy || !state.hasChanges}
                  onClick={state.reset}
                >
                  {t("resetChatChanges")}
                </Button>
                <Button disabled={isBusy || !state.hasChanges} onClick={() => void state.save()}>
                  {state.isSaving ? t("saving") : t("saveChatSelection")}
                </Button>
              </div>
              {state.isSaved && (
                <p className="chats-saved" role="status">
                  {t("chatSelectionSaved")}
                </p>
              )}
            </div>
          </>
        )}
      </section>
    </main>
  );
}
