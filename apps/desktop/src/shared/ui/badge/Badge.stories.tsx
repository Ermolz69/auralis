import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';
import { Badge } from './Badge';

const meta = {
  title: 'Shared UI/Badge',
  component: Badge,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    variant: {
      control: 'select',
      options: ['default', 'primary', 'accent', 'success', 'warning', 'danger', 'muted'],
    },
    size: { control: 'select', options: ['sm', 'md'] },
  },
} satisfies Meta<typeof Badge>;

export default meta;
type Story = StoryObj<typeof meta>;

// Default
export const Default: Story = {
  args: { children: 'Badge' },
};

// All Variants
export const AllVariants: Story = {
  render: () => (
    <div className="flex flex-wrap gap-3">
      <Badge variant="default">Default</Badge>
      <Badge variant="primary">Primary</Badge>
      <Badge variant="accent">Accent</Badge>
      <Badge variant="success">Success</Badge>
      <Badge variant="warning">Warning</Badge>
      <Badge variant="danger">Danger</Badge>
      <Badge variant="muted">Muted</Badge>
    </div>
  ),
};

// With Icon
export const WithIcon: Story = {
  args: {
    variant: 'success',
    icon: (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M20 6 9 17l-5-5" />
      </svg>
    ),
    children: 'Completed',
  },
};

// Long Text
export const LongText: Story = {
  args: {
    variant: 'warning',
    children: 'This is a very long badge text that might wrap',
    className: 'max-w-[150px]', // truncate works out of the box because of the span wrapper inside Badge
  },
};

// Small Dense
export const SmallDense: Story = {
  render: () => (
    <div className="flex gap-2 items-center">
      <Badge size="sm" variant="primary">
        New
      </Badge>
      <Badge size="sm" variant="accent">
        v1.2.0
      </Badge>
    </div>
  ),
};
