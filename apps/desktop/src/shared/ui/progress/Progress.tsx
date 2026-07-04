import React from 'react';

export interface ProgressProps extends React.HTMLAttributes<HTMLDivElement> {
  value?: number;
  max?: number;
  variant?: 'default' | 'success' | 'warning' | 'danger';
  indeterminate?: boolean;
}

export const Progress = React.forwardRef<HTMLDivElement, ProgressProps>(
  (
    { className = '', value = 0, max = 100, variant = 'default', indeterminate = false, ...props },
    ref
  ) => {
    const safeValue = Math.min(Math.max(value, 0), max);
    const percent = Math.round((safeValue / max) * 100);

    const baseContainer = 'relative h-2 w-full overflow-hidden rounded-full bg-surface border border-muted/50';
    
    const variants = {
      default: 'bg-primary',
      success: 'bg-success',
      warning: 'bg-warning',
      danger: 'bg-danger',
    };

    return (
      <div
        ref={ref}
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={max}
        aria-valuenow={indeterminate ? undefined : safeValue}
        className={`${baseContainer} ${className}`}
        {...props}
      >
        <div
          className={`h-full flex-1 transition-all duration-300 ease-in-out ${variants[variant]} ${
            indeterminate ? 'w-full animate-progress-indeterminate' : 'w-full'
          }`}
          style={
            indeterminate
              ? undefined
              : { transform: `translateX(-${100 - percent}%)` }
          }
        />
      </div>
    );
  }
);

Progress.displayName = 'Progress';
