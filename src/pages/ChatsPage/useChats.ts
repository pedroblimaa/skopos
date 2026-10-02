import { useContext, useEffect, useRef, useState } from "react";
import { appError, type AppMessage } from "../../app-message";
import { ChatsContext } from "./chats-context";

export function useChats() {
  const context = useContext(ChatsContext);
  if (!context) throw new Error("ChatsPage requires the authenticated chat cache");

  const cache = context;

  const [draft, setDraft] = useState<Set<string> | null>(null);
  const [saveError, setSaveError] = useState<AppMessage | null>(null);
  const [isSaved, setIsSaved] = useState(false);
  const active = useRef(true);
  const { load, saved } = cache;
  const selected = draft ?? new Set(saved.map((chat) => chat.id));

  useEffect(() => {
    active.current = true;
    void load();

    return () => {
      active.current = false;
    };
  }, [load]);

  function toggle(id: string) {
    setDraft((current) => {
      const next = new Set(current ?? saved.map((chat) => chat.id));
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
    setIsSaved(false);
  }

  function reset() {
    setDraft(null);
    setSaveError(null);
    setIsSaved(false);
  }

  async function save() {
    if (cache.isSaving) return;

    const selection = cache.chats.filter((chat) => selected.has(chat.id));
    setSaveError(null);
    setIsSaved(false);

    try {
      await cache.save(selection);
      if (active.current) {
        setDraft(null);
        setIsSaved(true);
      }
    } catch (reason) {
      if (active.current) setSaveError(appError(reason));
    }
  }

  const hasChanges = selected.size !== saved.length || saved.some((chat) => !selected.has(chat.id));
  const savedIds = new Set(saved.map((chat) => chat.id));
  const visibleChats = cache.chats.filter(
    (chat) => chat.available || selected.has(chat.id) || savedIds.has(chat.id),
  );

  return {
    chats: visibleChats,
    photos: cache.photos,
    selected,
    isLoading: cache.isLoading || (!cache.isLoaded && cache.error === null),
    isLoaded: cache.isLoaded,
    isSaving: cache.isSaving,
    error: saveError ?? cache.error,
    isSaved,
    hasChanges,
    toggle,
    reset,
    save,
    refresh: () => load(true),
    retry: () => (saveError ? save() : load(true)),
  };
}
