import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { AppIcon } from "../AppIcon/AppIcon";
import { WindowControls } from "./WindowControls";
import "./Titlebar.css";

export function Titlebar({ children, homePath }: { children?: ReactNode; homePath?: string }) {
  const brand = (
    <>
      <AppIcon />
      <span data-tauri-drag-region={homePath ? undefined : true}>Skopos</span>
    </>
  );

  return (
    <header className="app-titlebar" data-tauri-drag-region>
      {homePath ? (
        <Link className="app-titlebar-brand" to={homePath} draggable={false}>
          {brand}
        </Link>
      ) : (
        <div className="app-titlebar-brand" data-tauri-drag-region>
          {brand}
        </div>
      )}
      <div className="app-titlebar-drag" data-tauri-drag-region />
      <div className="app-titlebar-actions">{children}</div>
      <WindowControls />
    </header>
  );
}
