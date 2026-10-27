import assert from 'node:assert/strict';
import { youtubeDialog } from './navigation.mjs';

export async function smoke(page, report) {
  if (
    !(await report.step(
      'empty-projects',
      async () => {
        await page.getByText('Здесь пока нет материалов.', { exact: true }).waitFor();
        assert.equal(
          await page.getByRole('list', { name: 'Материалы библиотеки', exact: true }).count(),
          0,
        );
      },
      page,
    ))
  )
    return;
  if (
    !(await report.step(
      'input-validation',
      async () => {
        const dialog = await youtubeDialog(page);
        const input = dialog.getByLabel('Ссылка YouTube', { exact: true });
        await input.fill('not a URL');
        assert.equal(await input.evaluate((element) => element.validity.typeMismatch), true);
        await input.fill('');
        assert.equal(
          await dialog.getByRole('button', { name: 'Создать', exact: true }).isEnabled(),
          false,
        );
        return { scope: 'input-format-and-empty-value' };
      },
      page,
    ))
  )
    return;
  if (
    !(await report.step(
      'source-rejection',
      async () => {
        await page
          .getByLabel('Ссылка YouTube', { exact: true })
          .fill('https://example.invalid/not-youtube');
        const dialog = page.getByRole('dialog', { name: 'Добавить из YouTube', exact: true });
        await dialog.getByRole('button', { name: 'Создать', exact: true }).click();
        await dialog.getByRole('alert').waitFor();
        const ui_error = await dialog.getByRole('alert').innerText();
        await dialog.getByRole('button', { name: 'Отмена', exact: true }).click();
        await page.getByText('Здесь пока нет материалов.', { exact: true }).waitFor();
        return { ui_error, project_created: false };
      },
      page,
    ))
  )
    return;
  await report.step(
    'settings-and-return',
    async () => {
      await page.getByRole('button', { name: 'Настройки', exact: true }).click();
      await page.getByRole('dialog', { name: 'Настройки', exact: true }).waitFor();
      await page.getByRole('button', { name: 'Закрыть настройки', exact: true }).click();
      assert.equal(
        await page.getByText('Здесь пока нет материалов.', { exact: true }).isVisible(),
        true,
      );
    },
    page,
  );
}
