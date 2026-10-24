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
      'url-validation',
      async () => {
        await page
          .getByLabel('YouTube URL', { exact: true })
          .fill('https://example.invalid/not-youtube');
        await page.getByRole('button', { name: 'Add from YouTube', exact: true }).click();
        await page.getByRole('alert').waitFor();
        await page.getByText('No projects yet', { exact: true }).waitFor();
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
