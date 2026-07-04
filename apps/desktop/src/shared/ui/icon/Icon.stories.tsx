import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { Icon } from './Icon';
import { Button } from '../button';

const meta = {
  title: 'UI Kit/Icon',
  component: Icon,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
  argTypes: {
    name: { control: 'text' },
    size: { control: 'select', options: ['sm', 'md', 'lg'] },
    color: { control: 'select', options: ['default', 'primary', 'muted', 'danger', 'success', 'warning', 'accent'] },
  },
} satisfies Meta<typeof Icon>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: { name: 'Home' },
};

export const IconSizes: Story = {
  render: () => (
    <div className="flex items-end gap-4 text-text">
      <Icon name="Settings" size="sm" />
      <Icon name="Settings" size="md" />
      <Icon name="Settings" size="lg" />
      <Icon name="Settings" size={32} />
    </div>
  ),
};

export const IconColors: Story = {
  render: () => (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <Icon name="CircleCheck" color="success" />
        <span className="text-sm">Success</span>
      </div>
      <div className="flex items-center gap-2">
        <Icon name="TriangleAlert" color="warning" />
        <span className="text-sm">Warning</span>
      </div>
      <div className="flex items-center gap-2">
        <Icon name="OctagonX" color="danger" />
        <span className="text-sm">Danger</span>
      </div>
      <div className="flex items-center gap-2">
        <Icon name="Info" color="primary" />
        <span className="text-sm">Primary</span>
      </div>
      <div className="flex items-center gap-2">
        <Icon name="HelpCircle" color="muted" />
        <span className="text-sm">Muted</span>
      </div>
    </div>
  ),
};

export const IconInsideButton: Story = {
  render: () => (
    <div className="flex gap-4">
      <Button leftIcon={<Icon name="Download" size="sm" />}>Download</Button>
      <Button variant="danger" rightIcon={<Icon name="Trash2" size="sm" />}>
        Delete
      </Button>
    </div>
  ),
};

export const IconOnlyUsage: Story = {
  render: () => (
    <div className="flex items-center gap-4">
      <p className="text-sm text-muted">
        This icon button is accessible because it provides an ariaLabel for screen readers:
      </p>
      <button className="p-2 rounded hover:bg-surface transition-colors" title="Close modal">
        <Icon name="X" ariaLabel="Close modal" />
      </button>
    </div>
  ),
};
