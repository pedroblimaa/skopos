import { useCallback, useEffect, useRef, useState } from "react";
import { appError, type AppMessage } from "../../app-message";
import { telegram } from "../../telegram";
import type { TelegramChat } from "../../telegram.model";

export function useChatCache() {
  const [chats, setChats] = useState<TelegramChat[]>([]);
  const [saved, setSaved] = useState<TelegramChat[]>([]);
  const [photos, setPhotos] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<AppMessage | null>(null);
  const active = useRef(true);
  const loaded = useRef(false);
  const pending = useRef<Promise<void> | null>(null);
  const pendingSave = useRef<Promise<void> | null>(null);
  const photoGeneration = useRef(0);

  useEffect(() => {
    active.current = true;

    return () => {
      active.current = false;
      photoGeneration.current += 1;
    };
  }, []);

  const load = useCallback((refresh = false): Promise<void> => {
    if (pending.current) return pending.current;
    if (loaded.current && !refresh) return Promise.resolve();

    async function fetchChats() {
      setIsLoading(true);
      setError(null);

      try {
        const [availableResult, persistedResult] = await Promise.allSettled([
          telegram.listChats(),
          loaded.current ? Promise.resolve(null) : telegram.getSelectedChats(),
        ]);
        if (!active.current) return;
        if (availableResult.status === "rejected") throw availableResult.reason;
        if (persistedResult.status === "rejected") throw persistedResult.reason;

        const available = availableResult.value;
        const persisted = persistedResult.value;

        setChats((previous) => {
          const merged = new Map(available.map((chat) => [chat.id, chat]));
          for (const chat of persisted ?? previous) {
            if (!merged.has(chat.id)) merged.set(chat.id, { ...chat, available: false });
          }
          return [...merged.values()];
        });
        if (persisted !== null) setSaved(persisted);
        loaded.current = true;
        setIsLoaded(true);

        // Photos are optional and load independently after the chat list is ready.
        void loadPhotos(available, ++photoGeneration.current);
      } catch (reason) {
        if (active.current) setError(appError(reason));
      } finally {
        pending.current = null;
        if (active.current) setIsLoading(false);
      }
    }

    async function loadPhotos(available: TelegramChat[], generation: number) {
      let next = 0;

      function isCurrent() {
        return active.current && generation === photoGeneration.current;
      }

      async function worker() {
        while (isCurrent() && next < available.length) {
          const chat = available[next++];

          try {
            const photo = await telegram.getChatPhoto(chat.id);
            if (!isCurrent()) return;

            setPhotos((current) => {
              const updated = Object.fromEntries(
                Object.entries(current).filter(([id]) => id !== chat.id),
              );
              if (photo) updated[chat.id] = photo;
              return updated;
            });
          } catch {
            // An unavailable photo keeps the initials; it must not fail the chat list.
          }
        }
      }

      await Promise.all([worker(), worker()]);
    }

    pending.current = fetchChats();
    return pending.current;
  }, []);

  function save(selection: TelegramChat[]): Promise<void> {
    if (pendingSave.current) return pendingSave.current;

    setIsSaving(true);

    async function persist() {
      try {
        await telegram.saveSelectedChats(selection);
        if (active.current) setSaved(selection);
      } finally {
        pendingSave.current = null;
        if (active.current) setIsSaving(false);
      }
    }

    pendingSave.current = persist();
    return pendingSave.current;
  }

  return { chats, saved, photos, isLoading, isLoaded, isSaving, error, load, save };
}
