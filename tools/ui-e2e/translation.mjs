import { Blocked } from './core.mjs';
import { readyOrError, observeUntil, clickOnce } from './observe.mjs';
import { uiFailure } from './failures.mjs';

export async function setupTranslation(page, spec, remaining) {
  let installed = false,
    selected = false;
  const model = page.getByRole('region', { name: 'Модель перевода', exact: true });
  await model.waitFor();
  await observeUntil(
    async () => {
      if (
        await model
          .getByText('Этот экспериментальный пакет поддерживает только Windows x64.', {
            exact: true,
          })
          .isVisible()
      )
        throw new Blocked('Translation package supports Windows x64 only');
      const error = model.getByRole('alert');
      if (await error.isVisible()) throw uiFailure(await error.innerText(), 'Translation setup');
      return await model.getByRole('button').first().isVisible();
    },
    Math.min(30000, remaining()),
  );
  if (spec.install_translation === true) {
    const install = model.getByRole('button', { name: 'Скачать и установить', exact: true });
    if (await install.count()) {
      await clickOnce(install, () =>
        readyOrError(
          model.getByRole('button', { name: 'Выбрать модель', exact: true }),
          model,
          Math.min(900000, remaining()),
        ),
      );
      installed = true;
    }
    const select = model.getByRole('button', { name: 'Выбрать модель', exact: true });
    if (await select.count()) {
      await clickOnce(select, () =>
        readyOrError(
          model.getByText('Локальная модель выбрана.', { exact: false }),
          model,
          Math.min(120000, remaining()),
        ),
      );
      selected = true;
    }
    if (!installed || !selected)
      throw new Blocked(
        'Translator installation and selection were not both exercised through the UI; use a fresh profile without a seeded installation or selection',
      );
  }
  const ready = model.getByText('Локальная модель выбрана.', { exact: false });
  if (!(await ready.count())) throw new Blocked((await model.innerText()).slice(0, 3000));
  await ready.waitFor({ timeout: Math.min(120000, remaining()) });
  return {
    selected: true,
    installation_requested: spec.install_translation === true,
    installation_performed: installed,
    selection_performed: selected,
    inference_started: false,
  };
}
