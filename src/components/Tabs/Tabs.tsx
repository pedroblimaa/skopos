import type { ReactNode } from "react";
import "./Tabs.css";

export interface Tab<T extends string> {
  id: T;
  label: string;
  icon: ReactNode;
}

export function Tabs<T extends string>({
  tabs,
  active,
  onChange,
  disabled = false,
}: {
  tabs: Tab<T>[];
  active: T;
  onChange: (id: T) => void;
  disabled?: boolean;
}) {
  return (
    <div className="tabs" role="tablist" aria-label="Login method">
      {tabs.map(({ id, label, icon }) => (
        <button
          key={id}
          type="button"
          role="tab"
          aria-selected={active === id}
          disabled={disabled}
          className={`tabs__tab ${active === id ? "tabs__tab--active" : ""}`}
          onClick={() => {
            onChange(id);
          }}
        >
          {icon}
          {label}
        </button>
      ))}
    </div>
  );
}
