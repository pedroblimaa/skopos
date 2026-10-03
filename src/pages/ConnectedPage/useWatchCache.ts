import { useCallback, useEffect, useRef, useState } from "react";
import { telegram } from "../../telegram";
import type { Watch } from "../../watch.model";

export function useWatchCache() {
  const [watches, setWatches] = useState<Watch[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);
  const saved = useRef<Watch[] | null>(null);
  const pending = useRef<Promise<Watch[]> | null>(null);
  const active = useRef(true);

  useEffect(() => {
    active.current = true;

    return () => {
      active.current = false;
    };
  }, []);

  const load = useCallback((): Promise<Watch[]> => {
    if (saved.current) return Promise.resolve(saved.current);
    if (pending.current) return pending.current;

    async function fetchWatches() {
      try {
        const next = await telegram.listWatches();
        if (active.current) {
          saved.current = next;
          setWatches(next);
          setIsLoaded(true);
        }
        return next;
      } finally {
        pending.current = null;
      }
    }

    pending.current = fetchWatches();
    return pending.current;
  }, []);

  function update(watch: Watch) {
    if (!active.current || saved.current === null) return;

    const next = [watch, ...saved.current.filter((item) => item.id !== watch.id)].sort(
      (left, right) => right.id - left.id,
    );
    saved.current = next;
    setWatches(next);
  }

  function remove(id: number) {
    if (!active.current || saved.current === null) return;

    const next = saved.current.filter((watch) => watch.id !== id);
    saved.current = next;
    setWatches(next);
  }

  return { watches, isLoaded, load, update, remove };
}
