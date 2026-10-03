import { useState } from "react";
import { Button } from "../../components/Button/Button";
import { useLanguage } from "../../i18n/useLanguage";
import "./ClearResultsDialog.css";
import type { AppMessage } from "../../app-message";
import { FormField } from "../../components/FormField/FormField";
import { Dialog } from "../../components/Dialog/Dialog";

interface Props {
  isBusy: boolean;
  onClose: () => void;
  onClear: (before: number | null) => Promise<boolean>;
  error: AppMessage | null;
}

export function ClearResultsDialog({ isBusy, onClose, onClear, error }: Props) {
  const { t, message } = useLanguage();
  const [mode, setMode] = useState("all");
  const [cutoff, setCutoff] = useState("");
  const [hasError, setHasError] = useState(false);

  async function clear() {
    const before = mode === "all" ? null : Math.floor(new Date(cutoff).getTime() / 1000);
    if (before !== null && (!Number.isFinite(before) || before < 0)) return;
    setHasError(false);

    if (await onClear(before)) onClose();
    else setHasError(true);
  }

  return (
    <Dialog
      className="clear-results-dialog"
      title={t("clearResults")}
      isBusy={isBusy}
      onClose={onClose}
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void clear();
        }}
      >
        <fieldset disabled={isBusy}>
          <legend>{t("clearResultsChoice")}</legend>
          <label>
            <input
              type="radio"
              name="clear-mode"
              value="all"
              checked={mode === "all"}
              onChange={() => {
                setMode("all");
              }}
            />
            {t("clearAllResults")}
          </label>
          <label>
            <input
              type="radio"
              name="clear-mode"
              value="before"
              checked={mode === "before"}
              onChange={() => {
                setMode("before");
              }}
            />
            {t("clearBeforeDate")}
          </label>
          {mode === "before" && (
            <div className="clear-results-date">
              <FormField
                id="clear-results-cutoff"
                label={t("messageDateTime")}
                type="datetime-local"
                required
                min="1970-01-01T00:00"
                max="9999-12-31T23:59"
                value={cutoff}
                onChange={(event) => {
                  setCutoff(event.target.value);
                }}
              />
            </div>
          )}
        </fieldset>
        {hasError && (
          <p className="error" role="alert">
            {error === null ? t("clearResultsFailed") : message(error)}
          </p>
        )}
        <div className="dialog-actions">
          <Button type="button" variant="quiet" disabled={isBusy} onClick={onClose}>
            {t("cancel")}
          </Button>
          <Button type="submit" variant="danger" disabled={isBusy}>
            {isBusy ? t("clearingResults") : t("clearResults")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
