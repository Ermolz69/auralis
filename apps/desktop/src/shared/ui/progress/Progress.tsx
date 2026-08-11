import React from 'react';

export interface ProgressProps extends React.HTMLAttributes<HTMLDivElement> {
  value?: number;
  max?: number;
  label?: string;
  variant?: 'default' | 'success' | 'warning' | 'danger';
  indeterminate?: boolean;
}

export const Progress = React.forwardRef<HTMLDivElement, ProgressProps>(
  (
    {
      className = '',
      value = 0,
      max = 100,
      label,
      variant = 'default',
      indeterminate = false,
      ...props
    },
    ref,
  ) => {
    const safeMax = Math.max(max, 1);
    const safeValue = Math.min(Math.max(value, 0), safeMax);
    const percent = Math.round((safeValue / safeMax) * 100);
    const accessibleLabel = props['aria-label'] || label || 'Progress';

    const baseContainer = 'relative h-1 w-full overflow-hidden rounded-full bg-surface-hover';

    const variants = {
      default: 'bg-primary signal-glow-sm',
      success: 'bg-success',
      warning: 'bg-warning',
      danger: 'bg-danger',
    };

    return (
      <div
        ref={ref}
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={safeMax}
        aria-valuenow={indeterminate ? undefined : safeValue}
        aria-valuetext={indeterminate ? 'Loading' : `${percent}%`}
        aria-busy={indeterminate || undefined}
        aria-label={props['aria-labelledby'] ? undefined : accessibleLabel}
        className={`${baseContainer} ${className}`}
        {...props}
      >
        <div
          className={`h-full flex-1 transition-all duration-300 ease-in-out ${variants[variant]} ${
            indeterminate ? 'w-full animate-progress-indeterminate' : 'w-full'
          }`}
          style={indeterminate ? undefined : { transform: `translateX(-${100 - percent}%)` }}
        />
      </div>
    );
  },
);

Progress.displayName = 'Progress';
