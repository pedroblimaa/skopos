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
  const [isAutomaticSearch, setIsAutomaticSearch] = useState(false);
  const active = useRef(true);
  const pending = useRef(false);
  const loaded = useRef(false);
  const revision = useRef(0);
  const accountId = useRef<number | null>(null);
  const statusRevision = useRef(0);
  const idle = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    active.current = true;

    return () => {
      active.current = false;
      revision.current += 1;
    };
  }, []);

  const perform = useCallback(async (operation: () => Promise<SearchResults>, search = false) => {
    const isActive = () => active.current;

    if (pending.current || !isActive()) return false;

    const startedRevision = revision.current;
    pending.current = true;
    let release: () => void = () => {};
    idle.current = new Promise<void>((resolve) => {
      release = resolve;
    });
    setIsBusy(true);
    setIsSearching(search);
    setError(null);

    try {
      const next = await operation();

      if (!isActive() || startedRevision !== revision.current) return false;

      setResults(next);
      loaded.current = startedRevision === revision.current;
      setIsLoaded(true);

      return true;
    } catch (reason) {
      if (isActive()) setError(appError(reason));

      return false;
    } finally {
      pending.current = false;
      release();

      if (isActive()) {
        setIsBusy(false);
        setIsSearching(false);
      }
    }
  }, []);

  useEffect(() => {
    let isActive = true;
    const isCurrent = () => isActive;
    async function refresh(account: number) {
      if (!isActive || (accountId.current !== null && accountId.current !== account)) return;

      accountId.current = account;
      const nextRevision = ++revision.current;
      loaded.current = false;
      await idle.current;

      if (!isCurrent() || revision.current !== nextRevision) return;

      await perform(telegram.loadSearchResults);
    }

    const subscriptions = [
      telegram.onSearchUpdated((account) => {
        void refresh(account);
      }),
      telegram.onMonitoringStatus((status) => {
        if (!isActive || (accountId.current !== null && accountId.current !== status.accountId)) {
          return;
        }
        accountId.current = status.accountId;
        statusRevision.current += 1;
        setIsAutomaticSearch(status.isRunning);
      }),
    ];
    const startedRevision = revision.current;
    const startedStatusRevision = statusRevision.current;
    void telegram.monitoringStatus().then(
      (status) => {
        if (
          !isActive ||
          startedRevision !== revision.current ||
          startedStatusRevision !== statusRevision.current
        ) {
          return;
        }
        accountId.current = status.accountId;
        setIsAutomaticSearch(status.isRunning);
      },
      (reason: unknown) => {
        if (isActive) setError(appError(reason));
      },
    );

    for (const subscription of subscriptions) {
      void subscription.catch((reason: unknown) => {
        if (isActive) setError(appError(reason));
      });
    }

    return () => {
      isActive = false;

      for (const subscription of subscriptions) {
        void subscription.then(
          (unsubscribe) => {
            unsubscribe();
          },
          () => {},
        );
      }
    };
  }, [perform]);

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

  return {
    results,
    error,
    isBusy: isBusy || isAutomaticSearch,
    isSearching: isSearching || isAutomaticSearch,
    isLoaded,
    load,
    search,
    clear,
    invalidate,
  };
}
