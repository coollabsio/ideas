import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '~/lib/utils';

const buttonVariants = cva(
  'inline-flex h-8 min-w-fit cursor-pointer items-center justify-center gap-2 whitespace-nowrap rounded-sm border-2 px-2 text-sm normal-case outline-0 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-coollabs focus-visible:ring-offset-2 focus-visible:ring-offset-white disabled:cursor-not-allowed disabled:border-transparent disabled:bg-transparent disabled:text-neutral-300 disabled:hover:bg-transparent dark:focus-visible:ring-warning dark:focus-visible:ring-offset-base dark:disabled:text-neutral-600',
  {
    variants: {
      variant: {
        default:
          'border-neutral-200 bg-white text-black hover:bg-neutral-100 hover:text-black dark:border-coolgray-300 dark:bg-coolgray-100 dark:text-white dark:hover:bg-coolgray-200 dark:hover:text-white',
        highlighted:
          'border-coollabs bg-coollabs-50 text-coollabs-200 hover:bg-coollabs hover:text-white dark:border-coollabs-100 dark:bg-coollabs/20 dark:text-white dark:hover:bg-coollabs-100 dark:hover:text-white',
        ghost:
          'border-transparent bg-transparent text-neutral-700 hover:bg-neutral-100 hover:text-black dark:text-neutral-300 dark:hover:bg-coolgray-100 dark:hover:text-white',
      },
      size: {
        default: 'h-8 px-2',
        sm: 'h-8 px-2 text-sm',
        icon: 'h-8 w-8 p-0',
      },
    },
    defaultVariants: { variant: 'default', size: 'default' },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, type = 'button', ...props }, ref) => (
    <button
      ref={ref}
      type={type}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
);
Button.displayName = 'Button';

export { buttonVariants };
