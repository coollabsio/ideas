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
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/20 backdrop-blur-[2px] p-4"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={cardRef}
        className="relative flex w-full flex-col rounded-sm border border-coolgray-300 bg-base p-4 drop-shadow-sm lg:w-auto lg:min-w-[42rem] lg:max-w-4xl"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="mb-3 flex items-start justify-between gap-3">
          <div>
            <h2 id="dialog-title" className="text-base font-bold text-white">
              {title}
            </h2>
            {description && (
              <p className="mt-1 text-xs text-neutral-500">{description}</p>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="flex h-8 w-8 items-center justify-center rounded-full text-neutral-300 transition-colors hover:bg-coolgray-300 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-warning focus-visible:ring-offset-2 focus-visible:ring-offset-base"
          >
            <X className="h-6 w-6" strokeWidth={1.5} aria-hidden="true" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
