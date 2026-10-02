import { useId, useState, type ButtonHTMLAttributes, type ReactNode } from "react";
import { Info } from "lucide-react";
import "./InfoTooltip.css";

interface Props {
  label: string;
  children: ReactNode;
  trigger?: ReactNode;
  buttonProps?: ButtonHTMLAttributes<HTMLButtonElement>;
}

export function InfoTooltip({ label, children, trigger, buttonProps }: Props) {
  const id = useId();
  const [isDismissed, setIsDismissed] = useState(false);

  return (
    <span
      className="info-tooltip"
      data-dismissed={isDismissed}
      onMouseEnter={() => {
        setIsDismissed(false);
      }}
      onFocus={() => {
        setIsDismissed(false);
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") setIsDismissed(true);
      }}
    >
      <button
        {...buttonProps}
        className={buttonProps?.className ?? "info-tooltip-trigger"}
        type="button"
        aria-label={label}
        aria-describedby={id}
      >
        {trigger ?? <Info size={17} aria-hidden="true" />}
      </button>
      <span className="info-tooltip-content" id={id} role="tooltip">
        {children}
      </span>
    </span>
  );
}
