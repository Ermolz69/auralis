import { expect, test } from 'playwright/test';

test('native dialog contains trusted keyboard navigation and restores focus on every close', async ({
  page,
}) => {
  await page.goto('/iframe.html?id=design-system-components-dialog--basic-dialog&viewMode=story');
  const trigger = page.getByRole('button', { name: 'Open Dialog', exact: true });
  await expect(trigger).toBeFocused();
  await page.keyboard.press('Enter');
  const dialog = page.getByRole('dialog', { name: 'Project details', exact: true });
  await expect(dialog).toBeVisible();
  const controls = dialog.getByRole('button');
  await expect(controls).toHaveCount(3);
  const cancel = dialog.getByRole('button', { name: 'Cancel', exact: true });
  const proceed = dialog.getByRole('button', { name: 'Continue', exact: true });
  const close = dialog.getByRole('button', { name: 'Close dialog', exact: true });
  await expect(cancel).toBeFocused();
  for (const [key, targets] of [
    ['Tab', [proceed, close, cancel]],
    ['Shift+Tab', [close, proceed, cancel]],
  ]) {
    for (const target of targets) {
      await page.keyboard.press(key);
      const focus = await dialog.evaluate((element) => ({
        inside: element.contains(document.activeElement),
        documentFocused: document.hasFocus(),
        activeTag: document.activeElement?.tagName,
      }));
      if (!focus.inside) {
        expect(focus).toEqual({ inside: false, documentFocused: false, activeTag: 'BODY' });
        await page.keyboard.press(key);
      }
      await expect(target).toBeFocused();
    }
  }
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toBeFocused();
  for (const closeName of ['Cancel', 'Close dialog']) {
    await page.keyboard.press('Enter');
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: closeName, exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(trigger).toBeFocused();
  }
});
