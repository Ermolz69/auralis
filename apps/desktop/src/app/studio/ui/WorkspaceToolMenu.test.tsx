import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { WorkspaceToolMenu } from './WorkspaceToolMenu';
afterEach(cleanup);
function setup(materialName?: string) {
  const onTool = vi.fn();
  const onImport = vi.fn();
  const onCreateText = vi.fn();
  render(
    <WorkspaceToolMenu
      materialName={materialName}
      onTool={onTool}
      onImport={onImport}
      onCreateText={onCreateText}
    />,
  );
  const trigger = screen.getByRole('button', { name: 'Открыть инструмент' });
  const open = () => fireEvent.click(trigger);
  return { onTool, onImport, onCreateText, trigger, open };
}
it('navigates every tool and restores trigger focus after each admitted action', () => {
  const controls = setup('Selected material');
  controls.open();
  expect(screen.getByText('Selected material')).toBeTruthy();
  const menu = screen.getByRole('menu');
  fireEvent.keyDown(menu, { key: 'End' });
  expect(document.activeElement?.textContent).toBe('Импорт файлов');
  fireEvent.keyDown(menu, { key: 'Home' });
  expect(document.activeElement?.textContent).toBe('Источник и субтитры');
  fireEvent.keyDown(menu, { key: 'ArrowDown' });
  expect(document.activeElement?.textContent).toBe('Текст и подготовка');
  fireEvent.keyDown(menu, { key: 'ArrowUp' });
  expect(document.activeElement?.textContent).toBe('Источник и субтитры');
  fireEvent.click(screen.getByRole('menuitem', { name: 'Синтез речи' }));
  expect(controls.onTool).toHaveBeenCalledWith('speech');
  expect(document.activeElement).toBe(controls.trigger);
  controls.open();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Новый текст' }));
  expect(controls.onCreateText).toHaveBeenCalledTimes(1);
  controls.open();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Импорт файлов' }));
  expect(controls.onImport).toHaveBeenCalledTimes(1);
});
it('dismisses without an operation on escape, outside pointer, resize, blur and toggle', () => {
  const controls = setup();
  for (const dismiss of [
    () => fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' }),
    () => fireEvent.pointerDown(document.body),
    () => fireEvent.resize(window),
    () => fireEvent.blur(window),
    controls.open,
  ]) {
    controls.open();
    expect(screen.getByRole('menu')).toBeTruthy();
    dismiss();
    expect(screen.queryByRole('menu')).toBeNull();
  }
  expect(controls.onTool).not.toHaveBeenCalled();
});
