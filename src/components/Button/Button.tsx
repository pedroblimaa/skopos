import type { ButtonHTMLAttributes, ReactNode, Ref } from "react";
import "./Button.css";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  ref?: Ref<HTMLButtonElement>;
  variant?: "primary" | "quiet" | "danger";
  iconOnly?: boolean;
}

export function Button({
  children,
  variant = "primary",
  iconOnly = false,
  className = "",
  ...props
}: Props) {
  return (
    <button
      className={`button button--${variant}${iconOnly ? " button--icon" : ""} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
