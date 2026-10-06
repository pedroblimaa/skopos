export interface Watch {
  id: number;
  phrases: string[];
  maxPriceCents: number | null;
  /** null follows 20% of the maximum; zero disables the minimum. */
  minPriceCents: number | null;
}

export interface CreateWatch {
  phrases: string[];
  maxPriceCents: number | null;
  /** null follows 20% of the maximum; zero disables the minimum. */
  minPriceCents: number | null;
}
