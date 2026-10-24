import assert from 'node:assert/strict';

export async function smoke(page, report) {
  if (
    !(await report.step(
      'empty-projects',
      async () => {
        await page.getByText('No projects yet', { exact: true }).waitFor();
        assert.equal(
          await page.getByRole('button', { name: 'Add from YouTube', exact: true }).isEnabled(),
          false,
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
        const input = page.getByLabel('YouTube URL', { exact: true });
        await input.fill('not a URL');
        assert.equal(await input.evaluate((element) => element.validity.typeMismatch), true);
        await input.fill('');
        assert.equal(
          await page.getByRole('button', { name: 'Add from YouTube', exact: true }).isEnabled(),
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
          .getByLabel('YouTube URL', { exact: true })
          .fill('https://example.invalid/not-youtube');
        await page.getByRole('button', { name: 'Add from YouTube', exact: true }).click();
        await page.getByRole('alert').waitFor();
        await page.getByText('No projects yet', { exact: true }).waitFor();
        return { ui_error: await page.getByRole('alert').innerText(), project_created: false };
      },
      page,
    ))
  )
    return;
  await report.step(
    'settings-and-return',
    async () => {
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      await page.getByRole('heading', { name: 'Settings', exact: true }).waitFor();
      const back = page.getByRole('button', { name: /Back/ });
      await back.click();
      await page.getByLabel('YouTube URL', { exact: true }).waitFor();
      assert.equal(await page.getByText('No projects yet', { exact: true }).isVisible(), true);
    },
    page,
  );
}
