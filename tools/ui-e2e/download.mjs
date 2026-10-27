import assert from 'node:assert/strict';
import { acquireDownloadAdmission } from './admission.mjs';
import { clickOnce, observeUntil, jobState, readyOrError } from './observe.mjs';
import { uiFailure } from './failures.mjs';
import { youtubeDialog, openMaterial } from './navigation.mjs';

export async function download(page, report, spec, remaining, admit = acquireDownloadAdmission) {
  let release;
  let success = false;
  try {
    if (
      !(await report.step(
        'url-import',
        async () => {
          release = await admit();
          const dialog = await youtubeDialog(page);
          await dialog.getByLabel('Ссылка YouTube', { exact: true }).fill(spec.source_url);
          const timeout = Math.min(900000, remaining());
          const error = dialog.getByRole('alert');
          await clickOnce(dialog.getByRole('button', { name: 'Создать', exact: true }), () =>
            observeUntil(async () => {
              if (await error.isVisible())
                throw uiFailure(await error.innerText(), 'YouTube source import');
              return !(await dialog.isVisible());
            }, timeout),
          );
          await openMaterial(page, 'video');
          await page.getByRole('region', { name: 'Connected video source', exact: true }).waitFor();
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
        await track.press('Space');
        assert.equal(await track.isChecked(), true);
        const history = page.getByRole('region', {
          name: 'История импорта субтитров',
          exact: true,
        });
        await clickOnce(page.getByRole('button', { name: 'Получить субтитры', exact: true }), () =>
          jobState(
            history,
            'Состояние импорта субтитров',
            ['готово'],
            Math.min(900000, remaining()),
          ),
        );
        await readyOrError(
          page.getByText('Субтитры получены', { exact: true }).first(),
          history,
          Math.min(30000, remaining()),
        );
      },
      page,
    );
  } finally {
    if (release && success) success = (await report.step('download-release', release)) && success;
  }
  return success;
}
