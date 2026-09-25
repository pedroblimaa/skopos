import type { InputHTMLAttributes } from "react";
import "./FormField.css";

interface Props extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  id: string;
}

export function FormField({ label, id, ...props }: Props) {
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <input id={id} {...props} />
    </div>
  );
}
