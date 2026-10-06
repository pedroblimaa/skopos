import { useLanguage } from "../../i18n/useLanguage";

interface PriceRangeProps {
  maximum: number | null;
  minimum: string | null;
  onChange: (value: string | null) => void;
  hasError: boolean;
}

export function PriceRange({ maximum, minimum, onChange, hasError }: PriceRangeProps) {
  const { t } = useLanguage();
  const automatic = Number.isFinite(maximum) ? Math.floor((maximum ?? 0) / 5) : 0;
  const value = minimum ?? (automatic / 100).toFixed(2).replace(".", ",");

  return (
    <details className="watch-price-range" open={hasError || undefined}>
      <summary>{t("priceRange")}</summary>
      <div className="watch-minimum">
        <label htmlFor="min-price">{t("minimumPrice")}</label>
        <div className="watch-price-input">
          <span aria-hidden="true">R$</span>
          <input
            id="min-price"
            inputMode="decimal"
            value={value}
            onChange={(event) => {
              onChange(event.target.value);
            }}
            onBlur={(event) => {
              if (!event.target.value.trim()) onChange(null);
            }}
            aria-invalid={hasError}
            aria-describedby={hasError ? "minimum-help minimum-error" : "minimum-help"}
          />
        </div>
        <p className="watch-help" id="minimum-help">
          {t("minimumPriceHelp")}
        </p>
        {hasError && (
          <p className="watch-field-error" id="minimum-error">
            {t("minimumPriceValidation")}
          </p>
        )}
      </div>
    </details>
  );
}
