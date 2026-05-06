import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '~/lib/utils';

const buttonVariants = cva(
  'inline-flex h-8 items-center justify-center gap-2 whitespace-nowrap rounded-sm border-2 px-2 text-sm font-medium outline-0 transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-warning focus-visible:ring-offset-2 focus-visible:ring-offset-base disabled:cursor-not-allowed disabled:opacity-50',
  {
    variants: {
      variant: {
        default:
          'border-coolgray-300 bg-coolgray-100 text-white hover:bg-coolgray-200',
        highlighted:
          'border-coollabs-100 bg-coollabs/20 text-white hover:bg-coollabs-100',
        ghost:
          'border-transparent bg-transparent text-neutral-300 hover:bg-coolgray-100 hover:text-white',
        accent:
          'border-warning/40 bg-warning/15 text-warning hover:bg-warning/25',
      },
      size: {
        default: 'h-8 px-2',
        sm: 'h-7 px-2 text-xs',
        icon: 'h-8 w-8 p-0',
      },
    },
    defaultVariants: { variant: 'default', size: 'default' },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button
      ref={ref}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
);
Button.displayName = 'Button';

export { buttonVariants };
