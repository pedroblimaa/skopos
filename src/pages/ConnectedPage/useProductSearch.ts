import { useCallback, useEffect, useRef, useState } from "react";
import { appError, type AppMessage } from "../../app-message";
import { telegram } from "../../telegram";
import type { SearchResults } from "../../promotion.model";

export function useProductSearch() {
  const [results, setResults] = useState<SearchResults>({ matches: [], summary: null });
  const [error, setError] = useState<AppMessage | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);
  const active = useRef(true);
  const pending = useRef(false);
  const loaded = useRef(false);
  const revision = useRef(0);

  useEffect(() => {
    active.current = true;

    return () => {
      active.current = false;
    };
  }, []);

  const perform = useCallback(async (operation: () => Promise<SearchResults>, search = false) => {
    const isActive = () => active.current;
    if (pending.current || !isActive()) return false;
    const startedRevision = revision.current;
    pending.current = true;
    setIsBusy(true);
    setIsSearching(search);
    setError(null);

    try {
      const next = await operation();
      if (!isActive()) return false;

      setResults(next);
      loaded.current = startedRevision === revision.current;
      setIsLoaded(true);
      return true;
    } catch (reason) {
      if (isActive()) setError(appError(reason));
      return false;
    } finally {
      pending.current = false;
      if (isActive()) {
        setIsBusy(false);
        setIsSearching(false);
      }
    }
  }, []);

  const load = useCallback(() => {
    if (loaded.current) return Promise.resolve(true);

    return perform(telegram.loadSearchResults);
  }, [perform]);

  function invalidate() {
    revision.current += 1;
    loaded.current = false;
    setError(null);
  }

  function search() {
    return perform(telegram.searchProducts, true);
  }

  function clear(before: number | null) {
    return perform(async () => {
      await telegram.clearSearchResults(before);
      return telegram.loadSearchResults();
    });
  }

  return { results, error, isBusy, isSearching, isLoaded, load, search, clear, invalidate };
}
