import * as React from 'react';
import { useEffect, useRef } from 'react';
import { X } from 'lucide-react';

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
}

export function Dialog({ open, onClose, title, description, children }: DialogProps): React.ReactElement | null {
  const cardRef = useRef<HTMLDivElement>(null);
  const previouslyFocused = useRef<HTMLElement | null>(null);
  const onCloseRef = useRef(onClose);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    if (!open) return;
    previouslyFocused.current = document.activeElement as HTMLElement | null;

    const onKey = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onCloseRef.current();
      }
    };
    document.addEventListener('keydown', onKey);

    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    const focusTarget =
      cardRef.current?.querySelector<HTMLElement>('input, textarea, select') ??
      cardRef.current?.querySelector<HTMLElement>('button, [tabindex]:not([tabindex="-1"])');
    focusTarget?.focus();

    return () => {
      document.removeEventListener('keydown', onKey);
      document.body.style.overflow = prevOverflow;
      previouslyFocused.current?.focus();
    };
  }, [open]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/20 p-4 backdrop-blur-xs"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={cardRef}
        role="document"
        className="relative flex w-full flex-col rounded-sm border border-neutral-200 bg-white p-4 drop-shadow-sm dark:border-coolgray-300 dark:bg-base lg:w-auto lg:min-w-[42rem] lg:max-w-4xl"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="mb-3 flex items-start justify-between gap-3">
          <div>
            <h2 id="dialog-title" className="text-base font-bold text-black dark:text-white">
              {title}
            </h2>
            {description && (
              <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-500">{description}</p>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="flex h-8 w-8 items-center justify-center rounded-full text-neutral-500 transition-colors hover:bg-neutral-100 hover:text-black focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-coollabs focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:text-neutral-300 dark:hover:bg-coolgray-300 dark:hover:text-white dark:focus-visible:ring-warning dark:focus-visible:ring-offset-base"
          >
            <X className="h-6 w-6" strokeWidth={1.5} aria-hidden="true" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
