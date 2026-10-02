import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Bell, ChevronRight, Plus, Trash2 } from "lucide-react";
import { Button } from "../../components/Button/Button";
import { telegram } from "../../telegram";
import type { Watch } from "../../watch.model";
import { appError, type AppMessage } from "../../app-message";
import { useLanguage } from "../../i18n/useLanguage";
import "./ConnectedPage.css";

const currency = new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" });

export function ConnectedPage() {
  const { t, message } = useLanguage();
  const navigate = useNavigate();
  const [watches, setWatches] = useState<Watch[]>([]);
  const [error, setError] = useState<AppMessage | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [deleteId, setDeleteId] = useState<number | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  useEffect(() => {
    let isActive = true;

    async function loadWatches() {
      try {
        const savedWatches = await telegram.listWatches();
        if (isActive) setWatches(savedWatches);
      } catch (reason) {
        if (isActive) setError(appError(reason));
      } finally {
        if (isActive) setIsLoading(false);
      }
    }

    void loadWatches();

    return () => {
      isActive = false;
    };
  }, []);

  async function deleteWatch(id: number) {
    setIsDeleting(true);
    setError(null);

    try {
      await telegram.deleteWatch(id);
      setWatches((current) => current.filter((watch) => watch.id !== id));
      setDeleteId(null);
    } catch (reason) {
      setError(appError(reason));
    } finally {
      setIsDeleting(false);
    }
  }

  return (
    <main className="connected-page">
      <section className="connected-watches" aria-labelledby="watches-heading">
        <div className="connected-watches-heading">
          <div>
            <h1 id="watches-heading">{t("products")}</h1>
          </div>
          <Button onClick={() => void navigate("/watches/new")}>
            <Plus size={17} aria-hidden="true" /> {t("addProduct")}
          </Button>
        </div>
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
            {watches.map((watch) => (
              <li key={watch.id}>
                <div className="connected-watch-row">
                  <Link className="connected-watch-link" to={`/watches/${String(watch.id)}/edit`}>
                    <span className="connected-watch-icon">
                      <Bell size={20} aria-hidden="true" />
                    </span>
                    <div className="connected-watch-details">
                      <strong>{watch.phrases[0]}</strong>
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
                    <ChevronRight className="connected-watch-arrow" size={18} aria-hidden="true" />
                  </Link>
                  <button
                    className="connected-delete"
                    type="button"
                    aria-label={t("deleteNamedProduct", { name: watch.phrases[0] ?? t("product") })}
                    title={t("deleteProduct")}
                    disabled={isDeleting}
                    onClick={() => {
                      setDeleteId(watch.id);
                    }}
                  >
                    <Trash2 size={17} aria-hidden="true" />
                  </button>
                </div>
                {deleteId === watch.id && (
                  <div className="connected-delete-confirm">
                    <p>{t("deleteProductConfirm")}</p>
                    <div>
                      <Button
                        variant="quiet"
                        disabled={isDeleting}
                        onClick={() => {
                          setDeleteId(null);
                        }}
                      >
                        {t("keepProduct")}
                      </Button>
                      <Button
                        variant="danger"
                        disabled={isDeleting}
                        onClick={() => void deleteWatch(watch.id)}
                      >
                        {isDeleting ? t("deleting") : t("deleteProduct")}
                      </Button>
                    </div>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
