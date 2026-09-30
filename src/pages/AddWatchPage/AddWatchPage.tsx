import { useEffect, useState, type SubmitEvent } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ChevronRight, Plus, Trash2 } from "lucide-react";
import { Button } from "../../components/Button/Button";
import { InfoTooltip } from "../../components/InfoTooltip/InfoTooltip";
import { Card } from "../../components/Card/Card";
import { errorMessage, telegram } from "../../telegram";
import { parsePriceCents } from "./price";
import "./AddWatchPage.css";

interface PhraseField {
  id: number;
  value: string;
}

export function AddWatchPage() {
  const navigate = useNavigate();
  const { watchId } = useParams();
  const isEditing = watchId !== undefined;
  const [isLoading, setIsLoading] = useState(isEditing);
  const [loadError, setLoadError] = useState("");
  const [phrases, setPhrases] = useState<PhraseField[]>([{ id: 0, value: "" }]);
  const [nextId, setNextId] = useState(1);
  const [price, setPrice] = useState("");
  const [submitted, setSubmitted] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [saveError, setSaveError] = useState("");
  const priceCents = parsePriceCents(price);
  const hasPhraseError = submitted && phrases.some(({ value }) => !value.trim());
  const hasPriceError = submitted && Number.isNaN(priceCents);

  useEffect(() => {
    if (!watchId) return;
    let isActive = true;
    async function loadWatch() {
      try {
        const watches = await telegram.listWatches();
        if (!isActive) return;
        const watch = watches.find((item) => String(item.id) === watchId);
        if (!watch) {
          setLoadError("This product no longer exists.");
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
        if (isActive) setLoadError(errorMessage(reason));
      } finally {
        if (isActive) setIsLoading(false);
      }
    }
    void loadWatch();
    return () => {
      isActive = false;
    };
  }, [watchId]);

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
    setSaveError("");

    if (phrases.some(({ value }) => !value.trim()) || Number.isNaN(priceCents)) return;

    setIsBusy(true);
    try {
      const input = {
        phrases: phrases.map(({ value }) => value.trim()),
        maxPriceCents: priceCents,
      };
      if (isEditing) await telegram.updateWatch(Number(watchId), input);
      else await telegram.createWatch(input);
      void navigate("/connected");
    } catch (error) {
      setSaveError(errorMessage(error));
      setIsBusy(false);
    }
  }

  return (
    <main className="watch-page">
      <div className="watch-layout">
        <nav className="watch-breadcrumb" aria-label="Breadcrumb">
          <ol>
            <li>
              <Link to="/connected">Products</Link>
            </li>
            <li>
              <ChevronRight size={14} aria-hidden="true" />
              <span aria-current="page">{isEditing ? "Edit product" : "Add product"}</span>
            </li>
          </ol>
        </nav>
        <h1>{isEditing ? "Edit product" : "Add product"}</h1>
        {isLoading && (
          <p className="watch-help" role="status">
            Loading product…
          </p>
        )}
        {loadError && (
          <p className="error" role="alert">
            {loadError}
          </p>
        )}
        {!isLoading && !loadError && (
          <Card>
            <form className="watch-form" onSubmit={(event) => void save(event)} noValidate>
              <fieldset className="watch-fields" disabled={isBusy}>
                <div className="watch-search-heading">
                  <h2>Search names</h2>
                  <InfoTooltip label="How search names match">
                    Every word in one name must appear in the message (AND). Any name can match
                    (OR). Extra words are okay.
                  </InfoTooltip>
                </div>

                <div className="watch-phrases">
                  {phrases.map((phrase, index) => (
                    <div className="watch-phrase" key={phrase.id}>
                      <div className="watch-phrase-heading">
                        <label htmlFor={`phrase-${String(phrase.id)}`}>
                          {index === 0 ? "Product name" : `Alternative name ${String(index)}`}
                        </label>
                        {index > 0 && (
                          <button
                            className="watch-remove"
                            type="button"
                            aria-label={`Remove alternative name ${String(index)}`}
                            onClick={() => {
                              setPhrases((current) => current.filter(({ id }) => id !== phrase.id));
                            }}
                          >
                            <Trash2 size={15} aria-hidden="true" /> Remove
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
                          index === 0 ? "e.g. Laptop Vivobook S14" : "Another product name"
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
                          Enter a product name.
                        </p>
                      )}
                    </div>
                  ))}
                </div>

                <button className="watch-add" type="button" onClick={addPhrase}>
                  <Plus size={17} aria-hidden="true" /> Add alternative name
                </button>

                <div className="watch-price">
                  <label htmlFor="max-price">
                    Maximum price <span>(optional)</span>
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
                      Enter a valid price greater than zero, for example 3.500,00.
                    </p>
                  )}
                </div>
              </fieldset>
              {saveError && (
                <p role="alert" className="error">
                  {saveError}
                </p>
              )}
              <div className="watch-actions">
                <Button type="button" variant="quiet" onClick={() => void navigate("/connected")}>
                  Cancel
                </Button>
                <Button
                  type="submit"
                  disabled={isBusy || (submitted && (hasPhraseError || hasPriceError))}
                >
                  {isBusy ? "Saving…" : isEditing ? "Save changes" : "Add product"}
                </Button>
              </div>
            </form>
          </Card>
        )}
      </div>
    </main>
  );
}
