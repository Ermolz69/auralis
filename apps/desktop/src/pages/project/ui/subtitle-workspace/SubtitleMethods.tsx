import { WorkspaceSection } from './WorkspaceSection';

const methods = [
  {
    title: 'YouTube-субтитры',
    description: 'Авторские или автоматически созданные дорожки',
    status: 'Активно',
    active: true,
  },
  { title: 'Импорт SRT', description: 'Импорт готового файла пока не подключён', status: 'Скоро' },
  {
    title: 'Speech-to-Text',
    description: 'Распознавание речи пока не подключено',
    status: 'Скоро',
  },
];

export function SubtitleMethods() {
  return (
    <WorkspaceSection title="Способ получения">
      <div className="space-y-2">
        {methods.map((method) => (
          <div
            key={method.title}
            className={`flex min-h-[50px] items-center justify-between gap-3 rounded-md border px-3 py-2.5 ${
              method.active ? 'border-primary/60 bg-primary/10' : 'border-border bg-surface-raised'
            }`}
          >
            <span>
              <span className="block text-xs font-semibold text-text">{method.title}</span>
              <span className="mt-0.5 block text-[11px] text-subtle">{method.description}</span>
            </span>
            <span
              className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-semibold ${
                method.active ? 'bg-primary/15 text-primary' : 'bg-surface text-subtle'
              }`}
            >
              {method.status}
            </span>
          </div>
        ))}
      </div>
    </WorkspaceSection>
  );
}
