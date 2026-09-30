export interface Watch {
  id: number;
  phrases: string[];
  maxPriceCents: number | null;
}

export interface CreateWatch {
  phrases: string[];
  maxPriceCents: number | null;
}
