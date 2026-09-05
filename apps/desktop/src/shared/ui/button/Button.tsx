import React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  fullWidth?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'primary',
      size = 'md',
      loading = false,
      fullWidth = false,
      leftIcon,
      rightIcon,
      className = '',
      disabled,
      children,
      ...props
    },
    ref,
  ) => {
    const baseStyles =
      'motion-control inline-flex cursor-pointer items-center justify-center gap-2 border font-medium focus:outline-none disabled:cursor-not-allowed disabled:opacity-45';

    // Visual variants
    const variants = {
      primary:
        'border-primary-action bg-primary-action text-primary-foreground hover:bg-primary-action-hover active:bg-primary-action-pressed',
      secondary:
        'border-border bg-surface-raised text-text hover:border-border-strong hover:bg-surface-hover active:bg-surface-active',
      ghost:
        'border-transparent bg-transparent text-muted hover:bg-surface-raised hover:text-text active:bg-surface-hover',
      danger:
        'border-danger/30 bg-danger-soft text-danger hover:border-danger/50 hover:bg-danger-action-hover',
    };

    // Sizing
    const sizes = {
      sm: 'rounded-sm px-3 py-1.5 text-xs',
      md: 'rounded-md px-4 py-2 text-sm',
      lg: 'rounded-md px-5 py-2.5 text-sm',
    };

    const widthStyle = fullWidth ? 'w-full' : '';

    return (
      <button
        ref={ref}
        type={props.type || 'button'}
        disabled={disabled || loading}
        aria-busy={loading || undefined}
        className={`${baseStyles} ${variants[variant]} ${sizes[size]} ${widthStyle} ${className}`}
        {...props}
      >
        {loading && (
          <svg
            aria-hidden="true"
            className="animate-spin -ml-1 mr-2 h-4 w-4 text-current"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            ></circle>
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            ></path>
          </svg>
        )}
        {!loading && leftIcon && <span className="inline-flex shrink-0">{leftIcon}</span>}
        {children}
        {!loading && rightIcon && <span className="inline-flex shrink-0">{rightIcon}</span>}
      </button>
    );
  },
);

Button.displayName = 'Button';
