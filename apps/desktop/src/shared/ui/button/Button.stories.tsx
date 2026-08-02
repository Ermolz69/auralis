import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';
import { Button } from './Button';
import { Icon } from '../icon';

const meta = {
  title: 'Shared UI/Button',
  component: Button,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    variant: { control: 'select', options: ['primary', 'secondary', 'ghost', 'danger'] },
    size: { control: 'select', options: ['sm', 'md', 'lg'] },
    disabled: { control: 'boolean' },
    loading: { control: 'boolean' },
    fullWidth: { control: 'boolean' },
  },
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;

// Basic
export const Primary: Story = {
  args: { variant: 'primary', children: 'Primary Action' },
};

// All Variants
export const AllVariants: Story = {
  render: () => (
    <div className="flex gap-4">
      <Button variant="primary">Primary</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="danger">Danger</Button>
    </div>
  ),
};

// All Sizes
export const AllSizes: Story = {
  render: () => (
    <div className="flex items-center gap-4">
      <Button size="sm">Small</Button>
      <Button size="md">Medium</Button>
      <Button size="lg">Large</Button>
    </div>
  ),
};

// Disabled
export const Disabled: Story = {
  args: { disabled: true, children: 'Disabled Action' },
};

// Loading
export const Loading: Story = {
  args: { loading: true, children: 'Submitting...' },
};

// With Icon
export const WithIcon: Story = {
  render: () => (
    <div className="flex gap-4">
      <Button leftIcon={<Icon name="Plus" size="sm" />}>Create</Button>
      <Button variant="secondary" rightIcon={<Icon name="ArrowRight" size="sm" />}>
        Continue
      </Button>
    </div>
  ),
};

export const InteractionStates: Story = {
  render: () => (
    <div className="flex flex-col gap-3">
      <div className="flex gap-3">
        <Button>Default</Button>
        <Button className="ring-2 ring-focus ring-offset-2 ring-offset-bg">Focus visible</Button>
        <Button className="translate-y-px">Pressed</Button>
      </div>
      <div className="flex gap-3">
        <Button loading>Saving changes</Button>
        <Button disabled>Disabled</Button>
      </div>
    </div>
  ),
};

// Full Width
export const FullWidth: Story = {
  parameters: { layout: 'padded' },
  render: () => (
    <div className="w-96 p-4 border border-muted rounded bg-bg">
      <Button fullWidth>Full Width Button</Button>
    </div>
  ),
};
