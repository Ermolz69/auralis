import { expect, it, vi } from 'vitest';
import { commandMenus, toolUnavailable } from './commands';
import { asset } from '../../../../tests/studio';
import type { LibraryKind, LibraryTool } from '@/shared/api/contracts/library';

it.each([
  ['source', 'video', undefined],
  ['source', 'audio', undefined],
  ['source', 'text', 'Выберите видео или аудио'],
  ['translate', 'subtitle', undefined],
  ['translate', 'video', 'Выберите субтитры'],
  ['text', 'text', undefined],
  ['speech', 'subtitle', undefined],
  ['speech', 'audio', 'Выберите текст или субтитры'],
  ['voices', 'video', undefined],
  ['results', 'text', undefined],
] as [LibraryTool, LibraryKind, string | undefined][])(
  'admits %s only for compatible %s material',
  (tool, kind, reason) => {
    expect(toolUnavailable(tool, { ...asset, kind })).toBe(reason);
  },
);
it('opens a material picker when a tool has no selected material', () => {
  expect(toolUnavailable('source')).toBeUndefined();
});
it('binds file, project, tool and asset actions to the supplied context', () => {
  const run = vi.fn();
  const target = { projectId: 'selected-project', assetId: asset.id, workspaceId: 'workspace' };
  const menus = commandMenus(target, asset, run);
  const visit = (items: typeof menus.file) => {
    for (const item of items) {
      if (item.children) visit(item.children);
      else {
        item.run!();
        expect(run).toHaveBeenLastCalledWith(item.id, target);
      }
    }
  };
  Object.values(menus).forEach(visit);
  expect(menus.context.at(-1)?.label).toBe('Убрать из проекта…');
  expect(menus.project.every((item) => !item.disabled)).toBe(true);
});
it('requires a project for project actions and gives standalone assets library removal', () => {
  const target = { projectId: null, assetId: null };
  const menus = commandMenus(target, undefined, vi.fn());
  expect(menus.project.every((item) => item.disabled === 'Выберите проект')).toBe(true);
  expect(menus.context.map((item) => item.id)).toEqual(['new-project', 'new-text', 'imports']);
  expect(commandMenus(target, asset, vi.fn()).context.at(-1)?.label).toBe('Удалить из библиотеки…');
});
it('offers project imports and tools when its background is targeted', () => {
  expect(
    commandMenus({ projectId: 'project', assetId: null }, undefined, vi.fn()).context.map(
      (item) => item.id,
    ),
  ).toEqual(['open-project', 'imports', 'new-text', 'tools', 'rename-project', 'delete-project']);
});
