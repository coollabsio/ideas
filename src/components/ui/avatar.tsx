import * as React from 'react';
import { cn } from '~/lib/utils';

export interface AvatarProps extends React.HTMLAttributes<HTMLSpanElement> {
  src?: string | null;
  alt?: string;
  size?: number;
  fallback?: string;
}

export const Avatar = React.forwardRef<HTMLSpanElement, AvatarProps>(
  ({ className, src, alt, size = 28, fallback, ...props }, ref) => (
    <span
      ref={ref}
      className={cn(
        'inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full border border-neutral-200 bg-neutral-100 text-xs font-bold text-neutral-500 dark:border-coolgray-300 dark:bg-coolgray-200 dark:text-neutral-300',
        className
      )}
      style={{ width: size, height: size }}
      {...props}
    >
      {src ? (
        <img src={src} alt={alt ?? ''} className="h-full w-full object-cover" />
      ) : (
        <span aria-hidden="true">{fallback?.slice(0, 1).toUpperCase() ?? '?'}</span>
      )}
    </span>
  )
);
Avatar.displayName = 'Avatar';
