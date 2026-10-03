import { createContext, useContext } from "react";
import type { useWatchCache } from "./useWatchCache";

export const WatchContext = createContext<ReturnType<typeof useWatchCache> | null>(null);

export function useWatches() {
  const context = useContext(WatchContext);
  if (!context) throw new Error("Product pages require the authenticated watch cache");

  return context;
}
