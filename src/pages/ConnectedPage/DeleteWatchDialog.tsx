import { Button } from "../../components/Button/Button";
import { Dialog } from "../../components/Dialog/Dialog";
import type { AppMessage } from "../../app-message";
import { useLanguage } from "../../i18n/useLanguage";

interface Props {
  name: string;
  isBusy: boolean;
  error: AppMessage | null;
  onClose: () => void;
  onDelete: () => Promise<void>;
}

export function DeleteWatchDialog({ name, isBusy, error, onClose, onDelete }: Props) {
  const { t, message } = useLanguage();

  return (
    <Dialog
      title={t("deleteNamedProduct", { name })}
      description={t("deleteProductConfirm")}
      isBusy={isBusy}
      onClose={onClose}
    >
      {error !== null && (
        <p className="error" role="alert">
          {message(error)}
        </p>
      )}
      <div className="dialog-actions">
        <Button variant="quiet" disabled={isBusy} onClick={onClose} autoFocus>
          {t("keepProduct")}
        </Button>
        <Button variant="danger" disabled={isBusy} onClick={() => void onDelete()}>
          {isBusy ? t("deleting") : t("deleteProduct")}
        </Button>
      </div>
    </Dialog>
  );
}
