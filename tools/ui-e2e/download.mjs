import assert from 'node:assert/strict';
import { acquireDownloadAdmission } from './admission.mjs';
import { Blocked } from './core.mjs';

export async function download(page, report, spec, remaining, admit = acquireDownloadAdmission) {
  let release;
  let success = false;
  try {
    if (
      !(await report.step(
        'url-import',
        async () => {
          release = await admit();
          await page.getByLabel('YouTube URL', { exact: true }).fill(spec.source_url);
          await page.getByRole('button', { name: 'Add from YouTube', exact: true }).click();
          const timeout = Math.min(900000, remaining());
          const error = page
            .getByRole('form', { name: 'Add YouTube source', exact: true })
            .getByRole('alert');
          await Promise.race([
            page
              .getByRole('region', { name: 'Connected video source', exact: true })
              .waitFor({ timeout }),
            error.waitFor({ timeout }).then(async () => {
              throw new Blocked(`YouTube source unavailable: ${await error.innerText()}`);
            }),
          ]);
          await page.getByRole('button', { name: /Текст, субтитры и озвучка/ }).click();
        },
        page,
      ))
    )
      return false;
    success = await report.step(
      'chinese-srt',
      async () => {
        const track = page.getByRole('radio', { name: /Китайский.*SRT.*авторские/i });
        await track.waitFor({ state: 'attached', timeout: Math.min(120000, remaining()) });
        assert.equal(await track.count(), 1, 'Expected exactly one manual Chinese SRT track');
        await track.locator('..').click();
        assert.equal(await track.isChecked(), true);
        await page.getByRole('button', { name: 'Получить субтитры', exact: true }).click();
        await page
          .getByText('Субтитры получены', { exact: true })
          .first()
          .waitFor({ timeout: Math.min(900000, remaining()) });
        await page
          .getByRole('region', { name: 'Исходные субтитры', exact: true })
          .getByRole('button', { name: 'Обновить исходники', exact: true })
          .click();
      },
      page,
    );
  } finally {
    if (release && success) success = (await report.step('download-release', release)) && success;
  }
  return success;
}
