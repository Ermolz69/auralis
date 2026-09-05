import type { ReactNode } from 'react';
import { Icon, type IconName } from '../icon';

export type StateViewProps = {
  icon?: IconName;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  tone?: 'neutral' | 'danger';
  density?: 'default' | 'compact';
  loading?: boolean;
  role?: 'status' | 'alert';
  live?: 'polite' | 'assertive';
  className?: string;
};

export function StateView({
  icon,
  title,
  description,
  action,
  tone = 'neutral',
  density = 'default',
  loading = false,
  role,
  live,
  className = '',
}: StateViewProps) {
  const compact = density === 'compact';
  const toneClass = tone === 'danger' ? 'text-danger' : 'text-muted';

  return (
    <div
      className={`flex animate-content-in flex-col items-center justify-center text-center ${className}`}
      role={role}
      aria-live={live}
      aria-busy={loading || undefined}
    >
      {icon && (
        <Icon
          name={icon}
          size="lg"
          className={`${compact ? 'mb-3' : 'mb-4'} ${tone === 'danger' ? 'text-danger' : 'text-muted/50'}`}
        />
      )}
      <p
        className={`${compact ? 'text-sm' : 'text-lg'} font-medium text-text ${loading ? 'animate-pulse' : ''}`}
      >
        {title}
      </p>
      {description && (
        <div className={`${compact ? 'mt-1 text-xs' : 'mt-2 max-w-sm text-sm'} ${toneClass}`}>
          {description}
        </div>
      )}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
