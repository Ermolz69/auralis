import type { ReactNode } from 'react';
import { Icon, type IconName } from '../icon';

const toneClasses = {
  neutral: 'border-muted/40 bg-bg text-muted',
  accent: 'border-accent/40 bg-accent/10 text-accent-foreground',
  warning: 'border-warning/40 bg-warning/10 text-warning',
  danger: 'border-danger/40 bg-danger/10 text-danger',
} as const;

export type NoticeProps = {
  icon: IconName;
  title?: ReactNode;
  children: ReactNode;
  tone?: keyof typeof toneClasses;
  role?: 'status' | 'alert';
  live?: 'polite' | 'assertive';
  className?: string;
};

export function Notice({
  icon,
  title,
  children,
  tone = 'neutral',
  role,
  live,
  className = '',
}: NoticeProps) {
  return (
    <div
      className={`flex animate-surface-in gap-3 rounded-md border p-3 ${toneClasses[tone]} ${className}`}
      role={role}
      aria-live={live}
    >
      <Icon name={icon} size="sm" className="mt-0.5" />
      <div className="min-w-0">
        {title && <p className="text-sm font-medium text-text">{title}</p>}
        <div className="text-xs leading-snug">{children}</div>
      </div>
    </div>
  );
}
