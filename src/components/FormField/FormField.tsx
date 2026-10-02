import type { InputHTMLAttributes } from "react";
import { useLanguage } from "../../i18n/useLanguage";
import "./FormField.css";

interface Props extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  id: string;
}

export function FormField({ label, id, onInvalid, onInput, ...props }: Props) {
  const { t } = useLanguage();

  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <input
        id={id}
        {...props}
        onInvalid={(event) => {
          const input = event.currentTarget;
          input.setCustomValidity(
            t(input.validity.valueMissing ? "requiredField" : "invalidField"),
          );
          onInvalid?.(event);
        }}
        onInput={(event) => {
          event.currentTarget.setCustomValidity("");
          onInput?.(event);
        }}
      />
    </div>
  );
}
