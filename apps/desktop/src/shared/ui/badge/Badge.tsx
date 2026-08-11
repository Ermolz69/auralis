import React from 'react';

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'primary' | 'accent' | 'success' | 'warning' | 'danger' | 'muted';
  size?: 'sm' | 'md';
  icon?: React.ReactNode;
}

export const Badge = React.forwardRef<HTMLDivElement, BadgeProps>(
  ({ className = '', variant = 'default', size = 'md', icon, children, ...props }, ref) => {
    // Badges are non-interactive labels, no hover states or cursor-pointer
    const base =
      'inline-flex items-center justify-center rounded-full font-medium shrink-0 gap-1.5';

    const variants = {
      default: 'border border-border bg-surface-hover text-muted',
      primary: 'border border-primary/30 bg-primary-soft text-primary',
      accent: 'border border-accent/30 bg-accent-soft text-accent',
      success: 'border border-success/30 bg-success-soft text-success',
      warning: 'border border-warning/30 bg-warning-soft text-warning',
      danger: 'border border-danger/30 bg-danger-soft text-danger',
      muted: 'border border-border bg-surface-hover text-muted',
    };

    const sizes = {
      sm: 'px-2 py-0.5 text-xs',
      md: 'px-2.5 py-1 text-xs',
    };

    return (
      <div
        ref={ref}
        className={`${base} ${variants[variant]} ${sizes[size]} ${className}`}
        {...props}
      >
        {icon && <span className="shrink-0 flex">{icon}</span>}
        <span className="truncate">{children}</span>
      </div>
    );
  },
);

Badge.displayName = 'Badge';
