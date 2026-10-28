import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, waitFor, within } from 'storybook/test';
import { StudioStory } from './StudioShell.storyFixtures';
import { studioBoundary } from './StudioShell.ipc.storyFixtures';
let backend: ReturnType<typeof studioBoundary>;
const meta = {
  title: 'Product/Workspaces/Library and Tools',
  component: StudioStory,
  tags: ['autodocs'],
  beforeEach: ({ args }) => {
    backend = studioBoundary(args);
    return backend.restore;
  },
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component:
          'Real library persistence, workspace selection and tool tabs with model-free IPC fixtures.',
      },
    },
  },
} satisfies Meta<typeof StudioStory>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Library: Story = {
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole('heading', { name: 'Все материалы' })).toBeInTheDocument();
    await expect(canvas.queryByRole('tablist')).not.toBeInTheDocument();
    await expect(
      canvas.queryByRole('button', { name: /^Библиотека$|Очередь/ }),
    ).not.toBeInTheDocument();
    await expect(
      canvas.queryByRole('combobox', { name: 'Открытые рабочие пространства' }),
    ).not.toBeInTheDocument();
    await expect(
      canvas.getByRole('button', { name: 'Настройки' }).querySelector('svg'),
    ).toBeInTheDocument();
  },
};
export const EmptyLibrary: Story = { args: { empty: true } };
export const ConnectionRecovery: Story = {
  args: { failRefresh: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole('alert')).toHaveTextContent(
      'Library index is temporarily unavailable',
    );
    await userEvent.click(canvas.getByRole('button', { name: 'Повторить подключение' }));
    await expect(
      await canvas.findByRole('button', { name: /Вступление — русский текст/ }),
    ).toBeInTheDocument();
    await waitFor(() => expect(canvas.queryByRole('alert')).not.toBeInTheDocument());
  },
};

export const DraftAcrossToolTabs: Story = {
  args: { project: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const resources = await canvas.findByRole('complementary', {
      name: 'Материалы текущего проекта',
    });
    await userEvent.dblClick(
      within(resources).getByRole('button', { name: /Вступление — русский текст/ }),
    );
    const text = await canvas.findByRole('textbox', { name: 'Русский текст для озвучки' });
    await userEvent.clear(text);
    await userEvent.type(text, 'Сохранённый черновик главы.');
    await waitFor(() =>
      expect(backend.snapshot().workspaces[0].tabs[0].draft).toContain(
        'Сохранённый черновик главы.',
      ),
    );
    await userEvent.click(canvas.getByRole('menuitem', { name: 'Инструменты', exact: true }));
    await userEvent.click(
      await within(canvasElement.ownerDocument.body).findByRole('menuitem', {
        name: 'Синтез речи',
        exact: true,
      }),
    );
    const speech = await canvas.findByRole('tab', { name: /^Синтез речи/ });
    await expect(speech).toHaveAttribute('aria-selected', 'true');
    await expect(canvas.getAllByRole('tab')).toHaveLength(2);
    await userEvent.click(canvas.getByRole('menuitem', { name: 'Инструменты', exact: true }));
    await userEvent.click(
      await within(canvasElement.ownerDocument.body).findByRole('menuitem', {
        name: 'Синтез речи',
        exact: true,
      }),
    );
    await expect(canvas.getAllByRole('tab')).toHaveLength(2);
    const tablist = canvas.getByRole('tablist', { name: 'Вкладки рабочего пространства' });
    await expect(tablist.getAttribute('aria-owns')?.split(' ')).toEqual(
      canvas.getAllByRole('tab').map((tab) => tab.id),
    );
    await expect(within(tablist).queryByRole('button')).not.toBeInTheDocument();
    await expect(canvas.getByRole('textbox', { name: 'Русский текст для озвучки' })).toHaveValue(
      'Сохранённый черновик главы.',
    );
    speech.focus();
    await userEvent.keyboard('{Home}');
    const preparation = canvas.getByRole('tab', { name: /^Текст и подготовка/ });
    await waitFor(() => expect(preparation).toHaveAttribute('aria-selected', 'true'));
    await expect(preparation).toHaveFocus();
    await userEvent.keyboard('{End}');
    await waitFor(() => expect(speech).toHaveAttribute('aria-selected', 'true'));
    await expect(speech).toHaveFocus();
    await userEvent.click(canvas.getByRole('button', { name: 'Закрыть Синтез речи', exact: true }));
    await expect(canvas.getAllByRole('tab')).toHaveLength(1);
    const confirm = fn(() => false);
    const previous = window.confirm;
    window.confirm = confirm;
    try {
      await userEvent.click(
        canvas.getByRole('button', { name: 'Закрыть Текст и подготовка', exact: true }),
      );
      await expect(confirm).toHaveBeenCalledOnce();
      await expect(canvas.getByRole('textbox', { name: 'Русский текст для озвучки' })).toHaveValue(
        'Сохранённый черновик главы.',
      );
      confirm.mockReturnValue(true);
      await userEvent.click(
        canvas.getByRole('button', { name: 'Закрыть Текст и подготовка', exact: true }),
      );
      await waitFor(() => expect(canvas.queryByRole('tab')).not.toBeInTheDocument());
    } finally {
      window.confirm = previous;
    }
    await userEvent.dblClick(
      within(resources).getByRole('button', { name: /Вступление — русский текст/ }),
    );
    await expect(
      await canvas.findByRole('textbox', { name: 'Русский текст для озвучки' }),
    ).toHaveValue('Текст можно открыть без проекта и использовать нужные инструменты.');
    const restored = canvas.getByRole('textbox', { name: 'Русский текст для озвучки' });
    await userEvent.clear(restored);
    await userEvent.type(restored, 'Сохранённый черновик главы.');
    await waitFor(() =>
      expect(backend.snapshot().workspaces[0].tabs[0].draft).toContain(
        'Сохранённый черновик главы.',
      ),
    );
  },
};

export const MaterialContextRecovery: Story = {
  args: { project: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const resources = await canvas.findByRole('complementary', {
      name: 'Материалы текущего проекта',
    });
    backend.failContext(true);
    await userEvent.dblClick(
      within(resources).getByRole('button', { name: /Вступление — русский текст/ }),
    );
    await expect(await canvas.findByRole('alert')).toHaveTextContent(
      'Material context is temporarily unavailable',
    );
    await expect(
      canvas.queryByRole('textbox', { name: 'Русский текст для озвучки' }),
    ).not.toBeInTheDocument();
    backend.failContext(false);
    await userEvent.click(canvas.getByRole('button', { name: 'Повторить проверку материала' }));
    await expect(
      await canvas.findByRole('textbox', { name: 'Русский текст для озвучки' }),
    ).toHaveValue('Текст можно открыть без проекта и использовать нужные инструменты.');
  },
};
export const ProjectWorkspace: Story = {
  args: { project: true, busy: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole('tablist', {}, { timeout: 15000 })).toBeInTheDocument();
    await expect(canvas.queryByRole('tab')).not.toBeInTheDocument();
    await expect(canvas.queryByRole('button', { name: 'Синтез речи' })).not.toBeInTheDocument();
    const resources = canvas.getByRole('complementary', { name: 'Материалы текущего проекта' });
    await expect(
      within(resources).getByRole('button', { name: /Вступление — русский текст/ }),
    ).toBeInTheDocument();
    await userEvent.click(
      within(resources).getByRole('button', { name: /Вступление — русский текст/ }),
    );
    await userEvent.click(canvas.getByRole('menuitem', { name: 'Инструменты', exact: true }));
    const menu = await within(canvasElement.ownerDocument.body).findByRole('menu');
    await expect(within(menu).getAllByRole('menuitem')).toHaveLength(6);
    await expect(within(menu).getByRole('menuitem', { name: 'Синтез речи' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    const processes = canvas.getByRole('region', { name: 'Текущие процессы' });
    await expect(within(processes).getAllByRole('listitem')).toHaveLength(3);
    await expect(within(processes).queryByRole('button')).not.toBeInTheDocument();
    await expect(within(processes).getByText('В очереди')).toBeInTheDocument();
    await userEvent.click(canvas.getByRole('button', { name: 'Auralis — библиотека' }));
    await expect(await canvas.findByRole('heading', { name: 'Все материалы' })).toBeInTheDocument();
    await expect(within(processes).getAllByRole('listitem')).toHaveLength(3);
    await userEvent.dblClick(
      canvas.getByRole('button', { name: 'Рассказ · глава первая', exact: true }),
    );
    await expect(await canvas.findByRole('tablist', {}, { timeout: 15000 })).toBeInTheDocument();
  },
};
