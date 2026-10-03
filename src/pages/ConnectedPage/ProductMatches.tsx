import { useState } from "react";
import { ChevronDown, Copy, Check } from "lucide-react";
import { Button } from "../../components/Button/Button";
import { useLanguage } from "../../i18n/useLanguage";
import type { ProductMatch } from "../../promotion.model";
import "./ProductMatches.css";
import { telegram } from "../../telegram";
import { PromotionText } from "./PromotionText";

const currency = new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" });

export function ProductMatches({ matches, name }: { matches: ProductMatch[]; name: string }) {
  const { t } = useLanguage();

  return (
    <details className="product-matches">
      <summary aria-label={t("matchesForProduct", { name, count: matches.length })}>
        <span>{t("productMatches", { count: matches.length })}</span>
        <ChevronDown size={14} aria-hidden="true" />
      </summary>
      <div className="product-matches-content">
        {matches.length === 0 && <p className="product-matches-empty">{t("noProductMatches")}</p>}
        {matches.length > 0 && (
          <ul className="product-match-list">
            {matches.map((match) => (
              <MatchRow
                key={`${match.message.chatId}:${String(match.message.messageId)}`}
                match={match}
              />
            ))}
          </ul>
        )}
      </div>
    </details>
  );
}

function MatchRow({ match }: { match: ProductMatch }) {
  const { t, language } = useLanguage();
  const [isCopied, setIsCopied] = useState(false);
  const [hasCopyError, setHasCopyError] = useState(false);
  const [hasLinkError, setHasLinkError] = useState(false);
  const { message, priceCents } = match;
  const date = new Date(message.postedAt * 1000);
  const preview = message.text
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .find((line) => line.length > 0);

  async function copyLink() {
    if (!message.messageLink) return;
    setHasCopyError(false);

    try {
      await navigator.clipboard.writeText(message.messageLink);
      setIsCopied(true);
    } catch {
      setHasCopyError(true);
    }
  }

  async function openLink(url: string) {
    setHasLinkError(false);

    try {
      await telegram.openPromotionLink(url);
    } catch {
      setHasLinkError(true);
    }
  }

  return (
    <li className="product-match-row">
      <div className="product-match-heading">
        <details className="product-match-details">
          <summary>
            <span className="product-match-price">
              <strong>
                {priceCents === null ? t("priceUnavailable") : currency.format(priceCents / 100)}
              </strong>
              <ChevronDown size={14} aria-hidden="true" />
            </span>
            <span className="product-match-source">
              {message.chatTitle} ·{" "}
              <time dateTime={date.toISOString()}>
                {date.toLocaleString(language, { dateStyle: "short", timeStyle: "short" })}
              </time>
            </span>
            {preview && <span className="product-match-preview">{preview}</span>}
          </summary>
          <div className="product-match-content">
            {message.image && (
              <img
                className="product-match-image"
                src={message.image}
                alt={t("promotionImage", { chat: message.chatTitle })}
                loading="lazy"
              />
            )}
            <PromotionText text={message.text} onOpenLink={(url) => void openLink(url)} />
            {hasLinkError && (
              <p role="alert" className="error">
                {t("openLinkFailed")}
              </p>
            )}
          </div>
        </details>
        {message.messageLink !== null && (
          <Button
            variant="quiet"
            iconOnly
            aria-label={isCopied ? t("linkCopied") : t("copyMessageLink")}
            title={isCopied ? t("linkCopied") : t("copyMessageLink")}
            onClick={() => void copyLink()}
          >
            {isCopied ? (
              <Check size={14} aria-hidden="true" />
            ) : (
              <Copy size={14} aria-hidden="true" />
            )}
          </Button>
        )}
      </div>
      {hasCopyError && (
        <p role="alert" className="error">
          {t("copyLinkFailed")}
        </p>
      )}
    </li>
  );
}
