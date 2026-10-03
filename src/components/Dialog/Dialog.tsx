import { useEffect, useId, useRef, type ReactNode } from "react";
import "./Dialog.css";

interface Props {
  title: string;
  description?: string;
  children: ReactNode;
  isBusy: boolean;
  onClose: () => void;
  className?: string;
}

export function Dialog({ title, description, children, isBusy, onClose, className = "" }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const id = useId();

  useEffect(() => {
    const element = dialog.current;
    element?.showModal();

    return () => {
      element?.close();
    };
  }, []);

  return (
    <dialog
      ref={dialog}
      className={`dialog-panel ${className}`}
      aria-labelledby={`${id}-title`}
      aria-describedby={description ? `${id}-description` : undefined}
      aria-busy={isBusy}
      onCancel={(event) => {
        event.preventDefault();
        if (!isBusy) onClose();
      }}
    >
      <h2 id={`${id}-title`}>{title}</h2>
      {description && <p id={`${id}-description`}>{description}</p>}
      {children}
    </dialog>
  );
}
