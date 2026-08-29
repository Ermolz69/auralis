import { Icon } from '@/shared/ui/icon';

export function SectionLabel({ label }: { label: string }) {
  return (
    <div className="flex h-7 items-center px-2">
      <span className="flex-1 text-[10px] font-semibold uppercase tracking-wider text-subtle">
        {label}
      </span>
      <Icon name="ChevronDown" size={12} color="muted" />
    </div>
  );
}
