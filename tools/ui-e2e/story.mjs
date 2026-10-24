import assert from 'node:assert/strict';
import { Blocked } from './core.mjs';
import { playback } from './media.mjs';
import { requireQuiescent } from './processes.mjs';
import { download } from './download.mjs';
import { readyOrError } from './observe.mjs';

export const storySteps = [
  'url-import',
  'chinese-srt',
  'download-release',
  'source-identities',
  'translation-ready',
  'translate-edit-approve',
  ...['voxcpm2', 'espeech'].flatMap((engine) => [
    `${engine}-selection`,
    `${engine}-cancel-resume`,
    `${engine}-wav`,
    `${engine}-assembly`,
  ]),
  'verify-artifacts',
  'restart-playback',
];
export async function story(
  page,
  report,
  spec,
  remaining,
  quiescent = requireQuiescent,
  admit,
  sourceCheck = async () => ({ simulated: true }),
) {
  const step = async (id, fn) => {
    page.setDefaultTimeout(Math.min(30000, remaining()));
    return report.step(id, fn, page);
  };
  if (!(await download(page, report, spec, remaining, admit))) return false;
  if (!(await step('source-identities', sourceCheck))) return false;
  if (
    !(await step('translation-ready', async () => {
      const model = page.getByRole('region', { name: 'Модель перевода', exact: true });
      if (!(await model.getByText('Локальная модель выбрана.', { exact: false }).count()))
        throw new Blocked((await model.innerText()).slice(0, 3000));
      if (spec.allow_inference !== true)
        throw new Blocked(
          'This frozen case permits readiness inspection only; inference requires pinned runtime assets and selections',
        );
      await page
        .getByRole('button', { name: 'Подготовить перевод с китайского', exact: true })
        .click();
      await page.getByRole('button', { name: 'Начать перевод', exact: true }).click();
      const translation = page.getByRole('region', { name: 'Перевод субтитров', exact: true });
      await Promise.race([
        readyOrError(page.getByLabel('Готовый перевод', { exact: true }), translation, remaining()),
        translation
          .getByRole('button', { name: 'Продолжить перевод', exact: true })
          .waitFor({ timeout: remaining() })
          .then(async () => {
            throw new Error(
              `Translation stopped before publication: ${await translation.innerText()}`,
            );
          }),
      ]);
    }))
  )
    return false;
  if (
    !(await step('translate-edit-approve', async () => {
      const review = page.getByRole('region', { name: 'Переводы проекта', exact: true });
      await review.getByRole('button', { name: 'Исправить перевод', exact: true }).first().click();
      await review.getByLabel('Строка перевода 1', { exact: true }).fill(spec.edit_text);
      await review.getByRole('button', { name: 'Сохранить новую версию', exact: true }).click();
      await review.getByText(spec.edit_text, { exact: true }).waitFor();
      await review.getByText('Решение ещё не принято', { exact: false }).waitFor();
      const attach = review.getByRole('button', { name: 'Использовать эту версию', exact: true });
      if (await attach.count()) await attach.click();
      await review.getByLabel('Проверяющий', { exact: true }).fill('Automated UI test');
      await review
        .getByLabel('Основание решения', { exact: true })
        .fill(
          'Workflow-only test of this exact edited version; linguistic quality is not accepted.',
        );
      await review.getByRole('button', { name: 'Одобрить для речи', exact: true }).click();
      await review.getByText('Одобрено:', { exact: false }).waitFor();
    }))
  )
    return false;
  const panel = page.getByRole('region', { name: 'Голоса синтеза речи', exact: true });
  const assembly = page.getByRole('region', { name: 'Сборка итогового аудио', exact: true });
  for (const selection of spec.engines) {
    const engine = selection.engine_id;
    if (
      !(await step(`${engine}-selection`, async () => {
        await quiescent();
        const engineControl = panel.getByLabel('Движок', { exact: true });
        if (!(await engineControl.count()))
          throw new Blocked((await panel.innerText()).slice(0, 3000));
        await engineControl.selectOption(engine);
        const profile = panel.getByLabel('Движок и профиль', { exact: true });
        if (!(await profile.isVisible()))
          await panel.getByText('Профиль движка', { exact: true }).click();
        await profile.selectOption(selection.profile_id);
        await panel.getByLabel('Голос', { exact: true }).selectOption(selection.voice_id);
        const reference = panel.getByLabel('Референс', { exact: true });
        const allowed = await reference
          .locator('option')
          .evaluateAll(
            (options, id) => options.some((option) => option.value === id && !option.disabled),
            selection.reference_id,
          );
        if (!allowed)
          throw new Blocked(
            `Reference ${selection.voice_id}/${selection.reference_id} is not available as qualified`,
          );
        await reference.selectOption(selection.reference_id);
        await panel.getByRole('button', { name: 'Сохранить голос проекта', exact: true }).click();
        await panel.getByRole('button', { name: 'Субтитры', exact: true }).click();
        await panel.getByRole('button', { name: 'Обновить источники', exact: true }).click();
        const source = panel.getByLabel('Источник субтитров', { exact: true });
        await source
          .locator('option[value^="translation:"]')
          .first()
          .waitFor({ state: 'attached' });
        const translated = await source
          .locator('option')
          .evaluateAll((options) =>
            options
              .filter((option) => option.value.startsWith('translation:'))
              .map((option) => option.value),
          );
        assert.equal(translated.length, 1);
        await source.selectOption(translated[0]);
        await panel.getByRole('button', { name: 'Показать текст', exact: true }).click();
        await panel
          .getByRole('button', { name: 'Использовать одобренный перевод', exact: true })
          .click();
        await panel.getByRole('button', { name: 'Подготовить озвучку', exact: true }).click();
        await panel.getByRole('button', { name: 'Озвучить', exact: true }).waitFor();
      }))
    )
      return false;
    if (
      !(await step(`${engine}-cancel-resume`, async () => {
        await panel.getByRole('button', { name: 'Озвучить', exact: true }).click({ clickCount: 2 });
        await panel.getByRole('button', { name: 'Отменить', exact: true }).click();
        await panel
          .getByRole('button', { name: 'Продолжить', exact: true })
          .waitFor({ timeout: Math.min(120000, remaining()) });
        await quiescent();
        await panel.getByRole('button', { name: 'Продолжить', exact: true }).click();
      }))
    )
      return false;
    if (
      !(await step(`${engine}-wav`, async () => {
        await readyOrError(
          panel.getByRole('button', { name: 'Прослушать WAV', exact: true }),
          panel,
          remaining(),
        );
        await quiescent();
        await panel.getByRole('button', { name: 'Прослушать WAV', exact: true }).click();
        return playback(page, 'Готовая озвучка');
      }))
    )
      return false;
    if (
      !(await step(`${engine}-assembly`, async () => {
        await panel.getByRole('button', { name: 'Обновить готовую озвучку', exact: true }).click();
        const raw = panel.getByLabel('Сохранённые запуски', { exact: true });
        const rawId = await raw.inputValue();
        await panel.getByLabel('Готовый исходный синтез', { exact: true }).selectOption(rawId);
        await assembly
          .getByRole('button', { name: 'Измерить и показать тайминг', exact: true })
          .click();
        await readyOrError(assembly.getByRole('checkbox'), assembly, Math.min(30000, remaining()));
        await assembly.getByRole('checkbox').check();
        await assembly.getByRole('button', { name: 'Подготовить сборку', exact: true }).click();
        await assembly.getByRole('button', { name: 'Собрать WAV', exact: true }).click();
        await readyOrError(
          assembly.getByRole('button', { name: 'Прослушать итоговый WAV', exact: true }),
          assembly,
          remaining(),
        );
        await quiescent();
        await assembly
          .getByRole('button', { name: 'Прослушать итоговый WAV', exact: true })
          .click();
        return playback(page, 'Итоговое аудио');
      }))
    )
      return false;
  }
  return true;
}
