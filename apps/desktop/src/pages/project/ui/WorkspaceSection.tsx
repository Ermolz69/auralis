import type { ReactNode } from 'react';

export function WorkspaceSection({
  title,
  action,
  children,
}: {
  title: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  const id = `workspace-${title.toLocaleLowerCase('ru-RU').replaceAll(' ', '-')}`;
  return (
    <section aria-labelledby={id}>
      <div className="mb-2 flex min-h-7 items-center justify-between gap-3">
        <h2 id={id} className="text-xs font-semibold text-muted">
          {title}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}
