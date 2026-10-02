import { Search } from "lucide-react";
import type { InputHTMLAttributes } from "react";
import "./SearchField.css";

interface Props extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  label: string;
}

export function SearchField({ label, ...props }: Props) {
  return (
    <div className="search-field">
      <Search size={18} aria-hidden="true" />
      <input {...props} type="search" aria-label={label} />
    </div>
  );
}
