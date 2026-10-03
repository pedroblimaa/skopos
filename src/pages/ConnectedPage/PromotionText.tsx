import type { ReactNode } from "react";

export function PromotionText({
  text,
  onOpenLink,
}: {
  text: string;
  onOpenLink: (url: string) => void;
}) {
  const content: ReactNode[] = [];
  let offset = 0;

  for (const match of text.matchAll(/https?:\/\/[^\s<>"']+/giu)) {
    const url = match[0].replace(/[.,;!?)\]}]+$/u, "");
    content.push(text.slice(offset, match.index));
    content.push(
      <a
        key={match.index}
        href={url}
        onClick={(event) => {
          event.preventDefault();
          onOpenLink(url);
        }}
      >
        {url}
      </a>,
    );
    offset = match.index + url.length;
  }

  content.push(text.slice(offset));
  return <p className="product-match-text">{content}</p>;
}
