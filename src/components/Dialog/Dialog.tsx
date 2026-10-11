import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import "./Dialog.css";

interface Props {
  title: string;
  headerAction?: ReactNode;
  initialFocus?: RefObject<HTMLElement | null>;
  description?: string;
  children: ReactNode | ((close: () => void) => ReactNode);
  isBusy: boolean;
  onClose: () => void;
  className?: string;
}

export function Dialog({
  title,
  headerAction,
  initialFocus,
  description,
  children,
  isBusy,
  onClose,
  className = "",
}: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const [isClosing, setIsClosing] = useState(false);
  const id = useId();

  useLayoutEffect(() => {
    const element = dialog.current;
    const body = content.current;

    if (!element || !body) return;

    element.showModal();
    initialFocus?.current?.focus();
    const style = getComputedStyle(element);
    const frame =
      parseFloat(style.paddingTop) +
      parseFloat(style.paddingBottom) +
      parseFloat(style.borderTopWidth) +
      parseFloat(style.borderBottomWidth);

    const resizeDuration = parseFloat(style.getPropertyValue("--motion-page"));
    let resizeTimer: number | undefined;

    function resize() {
      if (!element || !body) return;

      const height = `${String(body.offsetHeight + frame)}px`;

      if (element.style.height === height) return;

      if (element.style.height && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        window.clearTimeout(resizeTimer);
        element.dataset.resizing = "true";
        resizeTimer = window.setTimeout(() => {
          delete element.dataset.resizing;
        }, resizeDuration);
      }

      element.style.height = height;
    }

    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(body);

    return () => {
      observer.disconnect();
      window.clearTimeout(resizeTimer);
      delete element.dataset.resizing;
      element.close();
    };
  }, [initialFocus]);

  useEffect(() => {
    if (!isClosing || !dialog.current) return;

    const duration = parseFloat(
      getComputedStyle(dialog.current).getPropertyValue("--motion-feedback"),
    );
    const timer = window.setTimeout(onClose, duration);

    return () => {
      window.clearTimeout(timer);
    };
  }, [isClosing, onClose]);

  function close() {
    if (isBusy || isClosing) return;

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      onClose();

      return;
    }

    setIsClosing(true);
  }

  return (
    <dialog
      ref={dialog}
      className={`dialog-panel ${className}`}
      aria-labelledby={`${id}-title`}
      aria-describedby={description ? `${id}-description` : undefined}
      aria-busy={isBusy}
      data-closing={isClosing}
      onCancel={(event) => {
        event.preventDefault();
        close();
      }}
    >
      <div ref={content} className="dialog-content" inert={isClosing}>
        <div className="dialog-heading">
          <h2 id={`${id}-title`}>{title}</h2>
          {headerAction}
        </div>
        {description && <p id={`${id}-description`}>{description}</p>}
        {typeof children === "function" ? children(close) : children}
      </div>
    </dialog>
  );
}
