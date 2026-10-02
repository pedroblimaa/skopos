import type { ButtonHTMLAttributes, ReactNode } from "react";
import "./Button.css";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
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
