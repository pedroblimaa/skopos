import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Bell, ChevronRight, Plus, Trash2, SearchX } from "lucide-react";
import { InfoTooltip } from "../../components/InfoTooltip/InfoTooltip";
import { Button } from "../../components/Button/Button";
import { telegram } from "../../telegram";
import { appError, type AppMessage } from "../../app-message";
import { useLanguage } from "../../i18n/useLanguage";
import "./ConnectedPage.css";
import { useSearchResults } from "./search-context";
import { ProductMatches } from "./ProductMatches";
import { SearchStatus, SearchFeedback, SearchInformation } from "./SearchStatus";
import { useWatches } from "./watch-context";
import { DeleteWatchDialog } from "./DeleteWatchDialog";

const currency = new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" });

export function ConnectedPage() {
  const { t, message } = useLanguage();
  const search = useSearchResults();
  const { load } = search;
  const { isBusy: isSearchBusy, error: searchError } = search;
  const navigate = useNavigate();
  const watchCache = useWatches();
  const { watches, load: loadWatches } = watchCache;
  const [error, setError] = useState<AppMessage | null>(null);
  const isLoading = !watchCache.isLoaded && error === null;
  const [deleteId, setDeleteId] = useState<number | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<AppMessage | null>(null);

  useEffect(() => {
    if (!isSearchBusy && searchError === null) void load();
  }, [load, isSearchBusy, searchError]);

  useEffect(() => {
    let isActive = true;

    async function loadProducts() {
      try {
        await loadWatches();
      } catch (reason) {
        if (isActive) setError(appError(reason));
      }
    }

    void loadProducts();

    return () => {
      isActive = false;
    };
  }, [loadWatches]);

  async function deleteWatch(id: number) {
    setIsDeleting(true);
    setDeleteError(null);

    try {
      await telegram.deleteWatch(id);
      watchCache.remove(id);
      search.invalidate();
      void load();
      setDeleteId(null);
    } catch (reason) {
      setDeleteError(appError(reason));
    } finally {
      setIsDeleting(false);
    }
  }

  return (
    <main className="connected-page">
      <section className="connected-watches" aria-labelledby="watches-heading">
        <div className="connected-watches-heading">
          <div className="connected-product-title">
            <h1 id="watches-heading">{t("products")}</h1>
            <SearchInformation search={search} />
          </div>
          <div className="connected-product-actions">
            <SearchStatus search={search} isSearchDisabled={isLoading || watches.length === 0} />
          </div>
        </div>
        <SearchFeedback search={search} />
        {error !== null && (
          <p role="alert" className="error">
            {message(error)}
          </p>
        )}
        {isLoading && (
          <p className="connected-loading" role="status">
            {t("loadingProducts")}
          </p>
        )}
        {!isLoading && !error && watches.length === 0 && (
          <div className="connected-empty">
            <Bell size={28} aria-hidden="true" />
            <h2>{t("emptyProductsTitle")}</h2>
            <p>{t("emptyProductsHelp")}</p>
            <Button onClick={() => void navigate("/watches/new")}>
              <Plus size={17} aria-hidden="true" /> {t("addFirstProduct")}
            </Button>
          </div>
        )}
        {watches.length > 0 && (
          <ul className="connected-watch-list">
            {watches.map((watch) => {
              const matches = search.results.matches
                .filter((match) => match.watchId === watch.id)
                .sort((left, right) => right.message.postedAt - left.message.postedAt);
              const hasSearchResults =
                search.results.summary !== null || search.results.matches.length > 0;

              return (
                <li key={watch.id}>
                  <div className="connected-watch-row interactive-row">
                    <span className="connected-watch-icon">
                      <Bell size={20} aria-hidden="true" />
                    </span>
                    <div className="connected-watch-link">
                      <div className="connected-watch-details">
                        <div className="connected-watch-name">
                          <Link
                            className="connected-watch-edit"
                            to={`/watches/${String(watch.id)}/edit`}
                          >
                            <strong>{watch.phrases[0]}</strong>
                          </Link>
                          {hasSearchResults && matches.length === 0 && (
                            <InfoTooltip
                              align="start"
                              tone="empty"
                              label={t("noProductMatches")}
                              trigger={<SearchX size={17} aria-hidden="true" />}
                            >
                              {t("noProductMatches")}
                            </InfoTooltip>
                          )}
                        </div>
                        <span>
                          {watch.phrases.length > 1
                            ? t("multipleNames", { count: watch.phrases.length })
                            : t("oneName")}{" "}
                          ·{" "}
                          {watch.maxPriceCents === null
                            ? t("anyPrice")
                            : t("upToPrice", { price: currency.format(watch.maxPriceCents / 100) })}
                        </span>
                      </div>
                      <ChevronRight
                        className="connected-watch-arrow"
                        size={18}
                        aria-hidden="true"
                      />
                    </div>
                    <button
                      className="connected-delete"
                      type="button"
                      aria-label={t("deleteNamedProduct", {
                        name: watch.phrases[0] ?? t("product"),
                      })}
                      title={t("deleteProduct")}
                      disabled={isDeleting}
                      onClick={() => {
                        setDeleteError(null);
                        setDeleteId(watch.id);
                      }}
                    >
                      <Trash2 size={17} aria-hidden="true" />
                    </button>
                  </div>
                  {matches.length > 0 && (
                    <ProductMatches name={watch.phrases[0] ?? t("product")} matches={matches} />
                  )}
                  {deleteId === watch.id && (
                    <DeleteWatchDialog
                      name={watch.phrases[0] ?? t("product")}
                      isBusy={isDeleting}
                      error={deleteError}
                      onClose={() => {
                        setDeleteId(null);
                      }}
                      onDelete={() => deleteWatch(watch.id)}
                    />
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </main>
  );
}
