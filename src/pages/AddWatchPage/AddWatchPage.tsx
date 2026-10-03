import { useEffect, useRef, useState, type SubmitEvent } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ChevronRight, Plus, Trash2 } from "lucide-react";
import { Button } from "../../components/Button/Button";
import { InfoTooltip } from "../../components/InfoTooltip/InfoTooltip";
import { Card } from "../../components/Card/Card";
import { telegram } from "../../telegram";
import { parsePriceCents } from "./price";
import { appError, type AppMessage } from "../../app-message";
import { useLanguage } from "../../i18n/useLanguage";
import "./AddWatchPage.css";
import { useWatches } from "../ConnectedPage/watch-context";
import { useSearchResults } from "../ConnectedPage/search-context";

interface PhraseField {
  id: number;
  value: string;
}

export function AddWatchPage() {
  const { t, message } = useLanguage();
  const navigate = useNavigate();
  const watchCache = useWatches();
  const search = useSearchResults();
  const { load } = watchCache;
  const { watchId } = useParams();
  const isEditing = watchId !== undefined;
  const [isLoading, setIsLoading] = useState(isEditing);
  const [loadError, setLoadError] = useState<AppMessage | null>(null);
  const [phrases, setPhrases] = useState<PhraseField[]>([{ id: 0, value: "" }]);
  const [nextId, setNextId] = useState(1);
  const [price, setPrice] = useState("");
  const [submitted, setSubmitted] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [saveError, setSaveError] = useState<AppMessage | null>(null);
  const isActive = useRef(false);
  const priceCents = parsePriceCents(price);
  const hasPhraseError = submitted && phrases.some(({ value }) => !value.trim());
  const hasPriceError = submitted && Number.isNaN(priceCents);
  const saveLabel = isEditing ? t("saveChanges") : t("addProduct");

  useEffect(() => {
    isActive.current = true;

    return () => {
      isActive.current = false;
    };
  }, []);

  useEffect(() => {
    if (!watchId) return;

    let isActive = true;

    async function loadWatch() {
      try {
        const watches = await load();
        if (!isActive) return;

        const watch = watches.find((item) => String(item.id) === watchId);
        if (!watch) {
          setLoadError({ code: "productNotFound" });
          return;
        }

        setPhrases(watch.phrases.map((value, id) => ({ id, value })));
        setNextId(watch.phrases.length);
        setPrice(
          watch.maxPriceCents === null
            ? ""
            : (watch.maxPriceCents / 100).toFixed(2).replace(".", ","),
        );
      } catch (reason) {
        if (isActive) setLoadError(appError(reason));
      } finally {
        if (isActive) setIsLoading(false);
      }
    }

    void loadWatch();

    return () => {
      isActive = false;
    };
  }, [watchId, load]);

  function addPhrase() {
    setPhrases((current) => [...current, { id: nextId, value: "" }]);
    setNextId((current) => current + 1);
  }

  function updatePhrase(id: number, value: string) {
    setPhrases((current) => current.map((phrase) => (phrase.id === id ? { id, value } : phrase)));
  }

  async function save(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitted(true);
    setSaveError(null);

    if (phrases.some(({ value }) => !value.trim()) || Number.isNaN(priceCents)) return;

    setIsBusy(true);

    try {
      const input = {
        phrases: phrases.map(({ value }) => value.trim()),
        maxPriceCents: priceCents,
      };

      const watch = isEditing
        ? await telegram.updateWatch(Number(watchId), input)
        : await telegram.createWatch(input);

      watchCache.update(watch);
      search.invalidate();

      if (!isActive.current) return;

      void navigate("/connected");
    } catch (error) {
      if (!isActive.current) return;

      setSaveError(appError(error));
      setIsBusy(false);
    }
  }

  return (
    <main className="watch-page">
      <div className="watch-layout">
        <nav className="watch-breadcrumb" aria-label={t("breadcrumb")}>
          <ol>
            <li>
              <Link to="/connected">{t("products")}</Link>
            </li>
            <li>
              <ChevronRight size={14} aria-hidden="true" />
              <span aria-current="page">{isEditing ? t("editProduct") : t("addProduct")}</span>
            </li>
          </ol>
        </nav>
        <h1>{isEditing ? t("editProduct") : t("addProduct")}</h1>
        {isLoading && (
          <p className="watch-help" role="status">
            {t("loadingProduct")}
          </p>
        )}
        {loadError !== null && (
          <p className="error" role="alert">
            {message(loadError)}
          </p>
        )}
        {!isLoading && !loadError && (
          <Card>
            <form className="watch-form" onSubmit={(event) => void save(event)} noValidate>
              <fieldset className="watch-fields" disabled={isBusy}>
                <div className="watch-search-heading">
                  <h2>{t("searchNames")}</h2>
                  <InfoTooltip label={t("searchNamesTooltip")}>{t("searchNamesHelp")}</InfoTooltip>
                </div>

                <div className="watch-phrases">
                  {phrases.map((phrase, index) => (
                    <div className="watch-phrase" key={phrase.id}>
                      <div className="watch-phrase-heading">
                        <label htmlFor={`phrase-${String(phrase.id)}`}>
                          {index === 0 ? t("productName") : t("alternativeName", { index })}
                        </label>
                        {index > 0 && (
                          <button
                            className="watch-remove"
                            type="button"
                            aria-label={t("removeAlternativeName", { index })}
                            onClick={() => {
                              setPhrases((current) => current.filter(({ id }) => id !== phrase.id));
                            }}
                          >
                            <Trash2 size={15} aria-hidden="true" /> {t("remove")}
                          </button>
                        )}
                      </div>
                      <input
                        id={`phrase-${String(phrase.id)}`}
                        value={phrase.value}
                        onChange={(event) => {
                          updatePhrase(phrase.id, event.target.value);
                        }}
                        placeholder={
                          index === 0 ? t("productNameExample") : t("anotherProductName")
                        }
                        aria-invalid={submitted && !phrase.value.trim()}
                        aria-describedby={
                          submitted && !phrase.value.trim()
                            ? `phrase-error-${String(phrase.id)}`
                            : undefined
                        }
                      />
                      {submitted && !phrase.value.trim() && (
                        <p className="watch-field-error" id={`phrase-error-${String(phrase.id)}`}>
                          {t("enterProductName")}
                        </p>
                      )}
                    </div>
                  ))}
                </div>

                <button className="watch-add" type="button" onClick={addPhrase}>
                  <Plus size={17} aria-hidden="true" /> {t("addAlternativeName")}
                </button>

                <div className="watch-price">
                  <label htmlFor="max-price">
                    {t("maximumPrice")} <span>{t("optional")}</span>
                  </label>
                  <div className="watch-price-input">
                    <span aria-hidden="true">R$</span>
                    <input
                      id="max-price"
                      inputMode="decimal"
                      value={price}
                      onChange={(event) => {
                        setPrice(event.target.value);
                      }}
                      placeholder="3.500,00"
                      aria-invalid={hasPriceError}
                      aria-describedby={hasPriceError ? "price-error" : undefined}
                    />
                  </div>
                  {hasPriceError && (
                    <p className="watch-field-error" id="price-error">
                      {t("priceValidation")}
                    </p>
                  )}
                </div>
              </fieldset>
              {saveError !== null && (
                <p role="alert" className="error">
                  {message(saveError)}
                </p>
              )}
              <div className="watch-actions">
                <Button type="button" variant="quiet" onClick={() => void navigate("/connected")}>
                  {t("cancel")}
                </Button>
                <Button
                  type="submit"
                  disabled={isBusy || (submitted && (hasPhraseError || hasPriceError))}
                >
                  {isBusy ? t("saving") : saveLabel}
                </Button>
              </div>
            </form>
          </Card>
        )}
      </div>
    </main>
  );
}
