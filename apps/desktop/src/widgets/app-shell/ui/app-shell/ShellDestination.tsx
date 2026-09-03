import { Icon, type IconName } from '@/shared/ui/icon';

type Props = {
  active: boolean;
  disabled?: boolean;
  icon: IconName;
  label: string;
  visualLabel: string;
  onClick: () => void;
  desktopHidden?: boolean;
};

export function ShellDestination(props: Props) {
  return (
    <button
      type="button"
      disabled={props.disabled}
      aria-current={props.active ? 'page' : undefined}
      aria-label={
        props.disabled ? `${props.label} unavailable without an active project` : props.label
      }
      title={props.disabled ? 'Open a project first' : undefined}
      onClick={props.onClick}
      className={`flex min-w-0 items-center justify-center gap-2 border-primary px-3 py-2 text-xs font-medium transition-colors lg:justify-start lg:border-l-2 lg:px-3 ${
        props.desktopHidden ? 'lg:hidden' : ''
      } ${
        props.active
          ? 'border-t-2 bg-primary/7 text-text lg:border-t-0'
          : 'border-t-2 border-transparent text-subtle hover:bg-surface hover:text-muted lg:border-t-0'
      } disabled:cursor-not-allowed disabled:opacity-45`}
    >
      <Icon name={props.icon} size={14} color={props.active ? 'primary' : 'muted'} />
      <span className="truncate">{props.visualLabel}</span>
    </button>
  );
}
