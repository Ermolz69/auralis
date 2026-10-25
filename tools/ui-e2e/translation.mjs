import { Blocked } from './core.mjs';
import { readyOrError } from './observe.mjs';

export async function setupTranslation(page, spec, remaining) {
  const model = page.getByRole('region', { name: 'Модель перевода', exact: true });
  await model.waitFor();
  await Promise.race([
    model
      .getByRole('button')
      .first()
      .waitFor({ timeout: Math.min(30000, remaining()) }),
    model
      .getByText('Этот экспериментальный пакет поддерживает только Windows x64.', { exact: true })
      .waitFor()
      .then(() => {
        throw new Blocked('Translation package supports Windows x64 only');
      }),
    model
      .getByRole('alert')
      .waitFor()
      .then(async () => {
        throw new Blocked(await model.getByRole('alert').innerText());
      }),
  ]);
  if (spec.install_translation === true) {
    const install = model.getByRole('button', { name: 'Скачать и установить', exact: true });
    if (await install.count()) {
      await install.click();
      await readyOrError(
        model.getByRole('button', { name: 'Выбрать модель', exact: true }),
        model,
        Math.min(900000, remaining()),
      );
    }
    const select = model.getByRole('button', { name: 'Выбрать модель', exact: true });
    if (await select.count()) {
      await select.click();
      await readyOrError(
        model.getByText('Локальная модель выбрана.', { exact: false }),
        model,
        Math.min(120000, remaining()),
      );
    }
  }
  const ready = model.getByText('Локальная модель выбрана.', { exact: false });
  if (!(await ready.count())) throw new Blocked((await model.innerText()).slice(0, 3000));
  await ready.waitFor({ timeout: Math.min(120000, remaining()) });
  return {
    selected: true,
    installation_requested: spec.install_translation === true,
    inference_started: false,
  };
}
