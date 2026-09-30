export function parsePriceCents(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;

  const normalized = trimmed.replace(/^R\$\s*/, "");
  if (!/^(?:\d+|\d{1,3}(?:\.\d{3})+)(?:,\d{1,2})?$/.test(normalized)) return NaN;

  const [reais, centavos = ""] = normalized.split(",");
  const cents = Number(reais.replace(/\./g, "")) * 100 + Number(centavos.padEnd(2, "0"));
  return Number.isSafeInteger(cents) && cents > 0 ? cents : NaN;
}
