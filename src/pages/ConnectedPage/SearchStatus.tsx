import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { History, ListRestart, ScanSearch, LoaderCircle, Plus } from "lucide-react";
import { InfoTooltip } from "../../components/InfoTooltip/InfoTooltip";
import { useLanguage } from "../../i18n/useLanguage";
import type { useProductSearch } from "./useProductSearch";
import { ClearResultsDialog } from "./ClearResultsDialog";

type Search = ReturnType<typeof useProductSearch>;

export function SearchStatus({
  search,
  isSearchDisabled,
}: {
  search: Search;
  isSearchDisabled: boolean;
}) {
  const { t } = useLanguage();
  const navigate = useNavigate();
  const [isClearOpen, setIsClearOpen] = useState(false);
  const { summary } = search.results;
  const hasResults = search.results.matches.length > 0 || summary !== null;

  return (
    <>
      {hasResults && (
        <InfoTooltip
          variant="action"
          label={t("clearResults")}
          trigger={<ListRestart size={18} aria-hidden="true" />}
          buttonProps={{
            className: "button button--quiet button--icon",
            disabled: search.isBusy,
            onClick: () => {
              setIsClearOpen(true);
            },
          }}
        >
          {t("clearResults")}
        </InfoTooltip>
      )}
      <InfoTooltip
        variant="action"
        label={t("addProduct")}
        trigger={<Plus size={19} aria-hidden="true" />}
        buttonProps={{
          className: "button button--quiet button--icon",
          onClick: () => void navigate("/watches/new"),
        }}
      >
        {t("addProduct")}
      </InfoTooltip>
      <InfoTooltip
        variant="action"
        label={t("searchNow")}
        trigger={
          search.isSearching ? (
            <LoaderCircle size={19} aria-hidden="true" className="connected-search-spinner" />
          ) : (
            <ScanSearch size={19} aria-hidden="true" />
          )
        }
        buttonProps={{
          className: "button button--primary button--icon",
          disabled: search.isBusy || isSearchDisabled,
          onClick: () => void search.search(),
        }}
      >
        {search.isSearching ? t("searchingProducts") : t("searchNow")}
      </InfoTooltip>
      {isClearOpen && (
        <ClearResultsDialog
          isBusy={search.isBusy}
          error={search.error}
          onClear={search.clear}
          onClose={() => {
            setIsClearOpen(false);
          }}
        />
      )}
    </>
  );
}

export function SearchInformation({ search }: { search: Search }) {
  const { t, language } = useLanguage();
  const { summary } = search.results;
  const date = (timestamp: number) =>
    new Date(timestamp * 1000).toLocaleString(language, { dateStyle: "short", timeStyle: "short" });

  return (
    <>
      {summary !== null && (
        <InfoTooltip
          align="start"
          label={t("lastSearch")}
          trigger={<History size={17} aria-hidden="true" />}
        >
          {t("lastSearchWindow", { since: date(summary.since), until: date(summary.startedAt) })}
        </InfoTooltip>
      )}
      <InfoTooltip align="start" label={t("savedResultsInfo")}>
        {t("savedResultsHelp")}
      </InfoTooltip>
    </>
  );
}

export function SearchFeedback({ search }: { search: Search }) {
  const { t, message } = useLanguage();
  const { summary } = search.results;
  const isIncomplete = summary !== null && summary.completedChats !== summary.totalChats;

  return (
    <div className="connected-search-status">
      <div role="status">
        {search.isSearching && <p>{t("searchingProducts")}</p>}
        {!search.isSearching && search.isBusy && !search.isLoaded && <p>{t("loadingMatches")}</p>}
        {!search.isBusy && isIncomplete && (
          <p>
            {t("searchIncomplete", {
              completed: summary.completedChats,
              total: summary.totalChats,
            })}
          </p>
        )}
      </div>
      {summary !== null && summary.unavailableChats.length > 0 && (
        <p>{t("searchUnavailableChats", { chats: summary.unavailableChats.join(", ") })}</p>
      )}
      {summary?.failure !== null && summary?.failure !== undefined && (
        <p role="alert" className="error">
          {message(summary.failure)}
        </p>
      )}
      {search.error !== null && (
        <p role="alert" className="error">
          {message(search.error)}
          {search.error.code === "searchNeedsChats" && (
            <>
              {" "}
              <Link to="/chats">{t("chooseSearchChats")}</Link>
            </>
          )}
        </p>
      )}
    </div>
  );
}
