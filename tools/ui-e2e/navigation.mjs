import assert from 'node:assert/strict';

export async function youtubeDialog(page) {
  await page
    .getByRole('menubar', { name: 'Главное меню' })
    .getByRole('menuitem', { name: 'Файл', exact: true })
    .click();
  await page.getByRole('menuitem', { name: 'Импорт', exact: true }).click();
  await page.getByRole('menuitem', { name: 'YouTube…', exact: true }).click();
  return page.getByRole('dialog', { name: 'Добавить из YouTube', exact: true });
}

export async function openMaterial(page, kind) {
  await page.getByRole('button', { name: 'Auralis — библиотека', exact: true }).click();
  await page.getByLabel('Тип материала', { exact: true }).selectOption(kind);
  const cards = page.getByRole('list', { name: 'Материалы библиотеки', exact: true });
  const item = cards
    .getByRole('button')
    .filter({ hasText: kind === 'subtitle' ? 'OriginalSubtitle' : 'Видео' });
  await item.waitFor();
  assert.equal(await item.count(), 1, `Expected exactly one owned source ${kind}`);
  await item.dblclick();
  await page.getByRole('main', { name: 'Рабочее пространство', exact: true }).waitFor();
}

export async function openTool(page, name) {
  await page
    .getByRole('menubar', { name: 'Главное меню' })
    .getByRole('menuitem', { name: 'Инструменты', exact: true })
    .click();
  await page.getByRole('menuitem', { name, exact: true }).click();
}

export async function openSubtitleTool(page, name) {
  await openMaterial(page, 'subtitle');
  await openTool(page, name);
}
