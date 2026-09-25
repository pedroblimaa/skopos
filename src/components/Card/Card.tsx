import type { ReactNode } from "react";
import "./Card.css";

export function Card({ children }: { children: ReactNode }) {
  return <section className="card">{children}</section>;
}
