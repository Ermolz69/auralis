import { expect } from 'playwright/test';
import { assertReachableControl } from './preview-quality.mjs';

export const cases = [
  {
    name: 'control-safety-dark-wide',
    story: 'design-system-foundations-control-safety-matrix--all-allowed-values',
    theme: 'dark',
    viewport: { width: 1440, height: 900 },
    validate: async (page) => {
      await expect(page.getByTestId('safe-button')).toHaveCount(14);
      await expect(page.getByTestId('safe-select')).toHaveCount(2);
    },
  },
  {
    name: 'control-safety-light-compact',
    story: 'design-system-foundations-control-safety-matrix--all-allowed-values',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      await expect(page.getByTestId('safe-icon').first()).toBeVisible();
      await expect(page.getByLabel('Large finite option list')).toHaveValue('0');
    },
  },
  {
    name: 'all-themes-gallery',
    story: 'design-system-foundations-theme-gallery--all-themes',
    theme: 'dark',
    viewport: { width: 1440, height: 900 },
    validate: async (page) => {
      const gallery = page.getByRole('region', { name: 'Auralis color theme gallery' });
      await expect(gallery).toBeVisible();
      await expect(gallery.getByRole('article')).toHaveCount(2);
    },
  },
  {
    name: 'long-project-title-wide',
    story: 'product-widgets-app-shell--with-long-project-title',
    theme: 'dark',
    viewport: { width: 1440, height: 900 },
    validate: async (page) => {
      await expect(page.getByRole('heading', { name: 'Workspace', exact: true })).toBeVisible();
    },
  },
  {
    name: 'long-project-title-compact-light',
    story: 'product-widgets-app-shell--with-long-project-title',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      await expect(page.getByRole('heading', { name: 'Workspace', exact: true })).toBeVisible();
    },
  },
  {
    name: 'home-empty',
    story: 'product-pages-home-states--local-first-empty',
    theme: 'dark',
    viewport: { width: 1280, height: 800 },
    validate: async (page) => {
      await expect(page.getByText('No projects yet', { exact: true })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Import local video' })).toBeEnabled();
    },
  },
  {
    name: 'home-loading',
    story: 'product-pages-home-states--loading-recent-projects',
    theme: 'light',
    viewport: { width: 1024, height: 768 },
    validate: async (page) => {
      await expect(page.getByRole('status')).toHaveText('Loading recent projects...');
      await expect(page.locator('[aria-busy="true"]')).toBeVisible();
    },
  },
  {
    name: 'project-list-error',
    story: 'product-features-project-list-states--fetch-error',
    theme: 'dark',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      await expect(page.getByRole('alert')).toContainText('Could not read the project index');
      await expect(page.getByRole('button', { name: 'Retry', exact: true })).toBeEnabled();
    },
  },
  {
    name: 'transcript-waiting',
    story: 'product-widgets-transcript-editor-states--waiting-for-pipeline',
    theme: 'light',
    viewport: { width: 1280, height: 800 },
    validate: async (page) => {
      await expect(page.getByText('Waiting for subtitles', { exact: true })).toBeVisible();
    },
  },
  {
    name: 'transcript-loading',
    story: 'product-widgets-transcript-editor-states--loading-transcript',
    theme: 'dark',
    viewport: { width: 1024, height: 768 },
    validate: async (page) => {
      await expect(page.getByRole('status')).toContainText('Loading transcript...');
      await expect(page.getByRole('status')).toHaveAttribute('aria-busy', 'true');
    },
  },
  {
    name: 'transcript-error',
    story: 'product-widgets-transcript-editor-states--error-with-retry',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      await expect(page.getByRole('alert')).toContainText('Transcript unavailable');
      await expect(page.getByRole('button', { name: 'Retry loading' })).toBeEnabled();
    },
  },
  {
    name: 'job-history-100-items',
    story: 'product-widgets-job-queue-panel-states--large-history',
    theme: 'dark',
    viewport: { width: 1024, height: 768 },
    validate: async (page) => {
      await expect(page.getByRole('listitem')).toHaveCount(100);
      const history = page.getByRole('list', { name: 'Operation history' });
      await expect(history).toBeVisible();
      expect(
        await history.evaluate((element) => {
          const scrollParent = element.parentElement?.parentElement;
          return Boolean(scrollParent && scrollParent.scrollHeight > scrollParent.clientHeight);
        }),
      ).toBe(true);
    },
  },
  {
    name: 'job-history-100-items-oldest-visible',
    story: 'product-widgets-job-queue-panel-states--large-history',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const oldestJob = page.getByRole('listitem').last();
      await oldestJob.scrollIntoViewIfNeeded();
      await expect(oldestJob).toContainText('Transcript generation 001');
      await expect(oldestJob).toBeVisible();
    },
  },
  ...['dark', 'light'].map((theme) => ({
    name: `extended-dialog-open-${theme}-compact`,
    story: 'design-system-components-dialog--basic-dialog',
    theme,
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const trigger = page.getByRole('button', { name: 'Open Dialog' });
      await expect(trigger).toBeFocused();
      await page.keyboard.press('Enter');
      const dialog = page.getByRole('dialog', { name: 'Project details' });
      await expect(dialog).toBeVisible();
      await expect(dialog).toHaveAccessibleDescription(
        'Review the source summary before continuing with the workspace.',
      );
      await assertReachableControl(dialog.getByRole('button', { name: 'Continue' }));
    },
  })),
  {
    name: 'extended-tabs-keyboard-media-light-compact',
    story: 'design-system-components-tabs--default-tabs',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const media = page.getByRole('tab', { name: 'Media', exact: true });
      await expect(media).toBeFocused();
      await expect(media).toHaveAttribute('aria-selected', 'true');
      await expect(page.getByRole('tabpanel', { name: 'Media' })).toHaveText('Source metadata');
      await assertReachableControl(media);
    },
  },
  {
    name: 'extended-input-long-labels-light-compact',
    story: 'design-system-components-input--long-labels-and-descriptions',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const input = page.getByRole('textbox', {
        name: 'Project source display name used by collaborators',
      });
      await expect(input).toHaveAccessibleDescription(
        'Long helper text wraps without changing the control relationship.',
      );
      await assertReachableControl(input);
      await expect(page.getByRole('textbox', { name: 'Transcript import URL' })).toHaveAttribute(
        'aria-invalid',
        'true',
      );
    },
  },
  {
    name: 'extended-select-error-dark-compact',
    story: 'design-system-components-select--error-state',
    theme: 'dark',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const select = page.getByRole('combobox', { name: 'Select Fruit' });
      await expect(select).toHaveAttribute('aria-invalid', 'true');
      await expect(select).toHaveAccessibleDescription(
        'This value controls the next import step. Select a source type to continue.',
      );
      await assertReachableControl(select);
    },
  },
  ...['dark', 'light'].map((theme) => ({
    name: `extended-studio-saved-draft-${theme}`,
    story: 'product-workspaces-library-and-tools--draft-across-tool-tabs',
    theme,
    viewport: theme === 'dark' ? { width: 1440, height: 900 } : { width: 800, height: 700 },
    validate: async (page) => {
      const tab = page.getByRole('tab', {
        name: 'Текст и подготовка · Вступление — русский текст',
        exact: true,
      });
      await expect(tab).toHaveAttribute('aria-selected', 'true');
      await expect(page.getByRole('tab')).toHaveCount(1);
      const text = page.getByRole('textbox', { name: 'Русский текст для озвучки', exact: true });
      await expect(text).toHaveValue('Сохранённый черновик главы.');
      await assertReachableControl(text);
      await assertReachableControl(
        page.getByRole('button', { name: 'Закрыть Текст и подготовка' }),
      );
    },
  })),
  {
    name: 'extended-job-history-recovered-light-compact',
    story: 'product-widgets-job-queue-panel-workflows--history-recovery-and-paging',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const history = page.getByRole('list', { name: 'Operation history' });
      await expect(history.getByRole('listitem')).toHaveCount(2);
      await expect(history).toContainText('Updated earlier import');
      await expect(history).toContainText('Older imported subtitles');
      await expect(page.getByRole('button', { name: 'Load older jobs' })).toHaveCount(0);
      await expect(page.getByRole('alert')).toHaveCount(0);
    },
  },
  {
    name: 'extended-media-missing-audio-dark-compact',
    story: 'product-widgets-media-panel-states--missing-audio',
    theme: 'dark',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      await expect(page.getByRole('alert')).toHaveText(
        'No audio tracks detected. Dubbing requires audio.',
      );
      await expect(page.getByLabel('Source: review.mov')).toBeVisible();
      await expect(page.getByText('1920×1080', { exact: true })).toBeVisible();
    },
  },
  {
    name: 'extended-progress-reduced-motion-light-compact',
    reducedMotion: 'reduce',
    story: 'design-system-components-progress--reduced-motion-fallback',
    theme: 'light',
    viewport: { width: 800, height: 700 },
    validate: async (page) => {
      const progress = page.getByRole('progressbar', { name: 'Reduced motion fallback progress' });
      await expect(progress).toHaveAttribute('aria-valuetext', 'Loading');
      await expect(progress).not.toHaveAttribute('aria-valuenow');
      expect(
        await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches),
      ).toBe(true);
      expect(
        await progress.evaluate(
          (element) => getComputedStyle(element.firstElementChild).animationName,
        ),
      ).toBe('none');
    },
  },
];
