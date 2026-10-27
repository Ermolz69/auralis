import { toolNames, type ActionTarget, type StudioAction } from '@/entities/library';
import type { LibraryAsset, LibraryTool } from '@/shared/api/contracts/library';
import type { MenuEntry } from '@/shared/ui/menu';
export function toolUnavailable(tool: LibraryTool, asset?: LibraryAsset): string | undefined {
  if (!asset || tool === 'voices' || tool === 'results') return;
  if (tool === 'source' && !['video', 'audio'].includes(asset.kind))
    return 'Выберите видео или аудио';
  if (tool === 'translate' && asset.kind !== 'subtitle') return 'Выберите субтитры';
  if (['text', 'speech'].includes(tool) && !['text', 'subtitle'].includes(asset.kind))
    return 'Выберите текст или субтитры';
}
export function commandMenus(
  target: ActionTarget,
  asset: LibraryAsset | undefined,
  run: (action: StudioAction, target: ActionTarget) => void,
) {
  const item = (id: StudioAction, label: string, disabled?: string): MenuEntry => ({
    id,
    label,
    disabled,
    run: () => run(id, target),
  });
  const tools = Object.entries(toolNames).map(([id, label]) =>
    item(`tool:${id as LibraryTool}`, label, toolUnavailable(id as LibraryTool, asset)),
  );
  const imports: MenuEntry = {
    id: 'imports',
    label: 'Импорт',
    children: [item('import', 'Файлы…'), item('youtube', 'YouTube…')],
  };
  const file = [
    item('new-project', 'Новый проект…'),
    item('new-text', 'Новый текст…'),
    imports,
    item('settings', 'Настройки…'),
    item('exit', 'Выход'),
  ];
  const project = [
    item(
      'open-project',
      'Открыть рабочее пространство',
      target.projectId ? undefined : 'Выберите проект',
    ),
    item('rename-project', 'Переименовать…', target.projectId ? undefined : 'Выберите проект'),
    item('delete-project', 'Удалить…', target.projectId ? undefined : 'Выберите проект'),
  ];
  const context = asset
    ? [
        item('open-asset', 'Открыть'),
        { id: 'tools', label: 'Открыть в инструменте', children: tools },
        item('attach', 'Добавить копию в проект…'),
        item('rename-asset', 'Переименовать…'),
        item('properties', 'Свойства…'),
        item('remove-asset', target.projectId ? 'Убрать из проекта…' : 'Удалить из библиотеки…'),
      ]
    : target.projectId
      ? [
          project[0],
          imports,
          item('new-text', 'Новый текст…'),
          { id: 'tools', label: 'Инструменты', children: tools },
          ...project.slice(1),
        ]
      : file.slice(0, 3);
  return { file, project, tools, context };
}
