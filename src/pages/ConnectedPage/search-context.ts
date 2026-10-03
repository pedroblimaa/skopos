import { createContext, useContext } from "react";
import type { useProductSearch } from "./useProductSearch";

export const SearchContext = createContext<ReturnType<typeof useProductSearch> | null>(null);

export function useSearchResults() {
  const context = useContext(SearchContext);
  if (!context) throw new Error("ConnectedPage requires the authenticated search context");

  return context;
}
