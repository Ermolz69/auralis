import React from 'react';

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'primary' | 'accent' | 'success' | 'warning' | 'danger' | 'muted';
  size?: 'sm' | 'md';
  icon?: React.ReactNode;
}

export const Badge = React.forwardRef<HTMLDivElement, BadgeProps>(
  ({ className = '', variant = 'default', size = 'md', icon, children, ...props }, ref) => {
    // Badges are non-interactive labels, no hover states or cursor-pointer
    const base = 'inline-flex items-center justify-center rounded-full font-medium shrink-0 gap-1.5';
    
    const variants = {
      default: 'bg-surface text-text border border-muted',
      primary: 'bg-primary/20 text-primary border border-primary/30',
      accent: 'bg-accent/20 text-accent border border-accent/30',
      success: 'bg-success/20 text-success border border-success/30',
      warning: 'bg-warning/20 text-warning border border-warning/30',
      danger: 'bg-danger/20 text-danger border border-danger/30',
      muted: 'bg-muted/10 text-muted border border-muted/20',
    };

    const sizes = {
      sm: 'px-2 py-0.5 text-xs',
      md: 'px-2.5 py-1 text-sm',
    };

    return (
      <div ref={ref} className={`${base} ${variants[variant]} ${sizes[size]} ${className}`} {...props}>
        {icon && <span className="shrink-0 flex">{icon}</span>}
        <span className="truncate">{children}</span>
      </div>
    );
  }
);

Badge.displayName = 'Badge';
