import { expect, userEvent, within } from 'storybook/test';

export async function playDefaultTabs({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const transcriptTab = canvas.getByRole('tab', { name: 'Transcript' });
  const mediaTab = canvas.getByRole('tab', { name: 'Media' });
  const transcriptPanel = canvas.getByRole('tabpanel', { name: 'Transcript' });

  await expect(transcriptTab).toHaveAttribute('aria-controls', transcriptPanel.id);
  await transcriptTab.focus();
  await userEvent.keyboard('{ArrowRight}');
  const mediaPanel = canvas.getByRole('tabpanel', { name: 'Media' });

  await expect(mediaTab).toHaveFocus();
  await expect(mediaTab).toHaveAttribute('aria-selected', 'true');
  await expect(mediaPanel).toBeVisible();
  await expect(mediaPanel).toHaveAttribute('aria-labelledby', mediaTab.id);
  await userEvent.keyboard('{End}');
  await expect(canvas.getByRole('tab', { name: 'Jobs' })).toHaveFocus();
  await userEvent.keyboard('{ArrowRight}');
  await expect(transcriptTab).toHaveFocus();
  await userEvent.keyboard('{ArrowLeft}');
  await expect(canvas.getByRole('tab', { name: 'Jobs' })).toHaveFocus();
  await userEvent.keyboard('{Home}{ArrowRight}');
  await expect(mediaTab).toHaveFocus();
  await expect(mediaPanel).toBeVisible();
  await expect(transcriptPanel).not.toBeVisible();
}

export async function playDisabledTab({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const disabledTab = canvas.getByRole('tab', { name: 'Export unavailable' });

  await expect(disabledTab).toBeDisabled();
  await expect(disabledTab).toHaveAttribute('aria-selected', 'false');
  const transcript = canvas.getByRole('tab', { name: 'Transcript' });
  const media = canvas.getByRole('tab', { name: 'Media' });
  await expect(transcript).toHaveAttribute('aria-selected', 'true');
  transcript.focus();
  await userEvent.keyboard('{ArrowLeft}');
  await expect(media).toHaveFocus();
  await expect(media).toHaveAttribute('aria-selected', 'true');
  await userEvent.keyboard('{ArrowRight}');
  await expect(transcript).toHaveFocus();
  await expect(disabledTab).toHaveAttribute('aria-selected', 'false');
}

export async function playControlledSelection({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const general = canvas.getByRole('tab', { name: 'General' });
  const advanced = canvas.getByRole('tab', { name: 'Advanced' });
  await userEvent.click(general);
  await userEvent.keyboard('{ArrowRight}');
  await expect(advanced).toHaveFocus();
  await expect(general).toHaveAttribute('aria-selected', 'true');
  await expect(canvas.getByRole('tabpanel', { name: 'General' })).toBeVisible();
  await expect(canvas.getByRole('status', { name: 'Requested view' })).toHaveTextContent(
    'advanced',
  );
  await expect(canvas.getByRole('status', { name: 'View requests' })).toHaveTextContent(/^2$/);
  await userEvent.click(canvas.getByRole('button', { name: 'Accept view' }));
  await expect(advanced).toHaveAttribute('aria-selected', 'true');
  await expect(canvas.getByRole('tabpanel', { name: 'Advanced' })).toBeVisible();
}

export async function playRetainedPanelState({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const field = canvas.getByRole('textbox', { name: 'Retained note' });
  await userEvent.clear(field);
  await userEvent.type(field, 'Edited note');
  await userEvent.click(canvas.getByRole('tab', { name: 'Source' }));
  await expect(field).not.toBeVisible();
  await userEvent.click(canvas.getByRole('tab', { name: 'Notes' }));
  await expect(field).toBeVisible();
  await expect(field).toHaveValue('Edited note');
  await userEvent.tab();
  await expect(canvas.getByRole('tabpanel', { name: 'Notes' })).toHaveFocus();
}

export async function playVerticalKeyboard({ canvasElement }: { canvasElement: HTMLElement }) {
  const canvas = within(canvasElement);
  const first = canvas.getByRole('tab', { name: 'First view' });
  const last = canvas.getByRole('tab', { name: 'Last view' });
  await expect(canvas.getByRole('tablist', { name: 'Vertical views' })).toHaveAttribute(
    'aria-orientation',
    'vertical',
  );
  await userEvent.click(first);
  await userEvent.keyboard('{ArrowDown}');
  await expect(last).toHaveFocus();
  await expect(canvas.getByRole('tabpanel', { name: 'Last view' })).toBeVisible();
  await userEvent.keyboard('{ArrowDown}');
  await expect(first).toHaveFocus();
  await userEvent.keyboard('{End}{Home}{ArrowUp}');
  await expect(last).toHaveFocus();
  await expect(canvas.getByRole('tab', { name: 'Unavailable view' })).toBeDisabled();
}
