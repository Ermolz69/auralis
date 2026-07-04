import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { Input } from './Input';

const meta = {
  title: 'UI Kit/Input',
  component: Input,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    helperText: { control: 'text' },
    error: { control: 'boolean' },
    disabled: { control: 'boolean' },
  },
} satisfies Meta<typeof Input>;

export default meta;
type Story = StoryObj<typeof meta>;

// Default
export const Default: Story = {
  args: { placeholder: 'Enter text here...' },
};

// With Label
export const WithLabel: Story = {
  args: { label: 'Username', placeholder: 'Enter your username' },
};

// With Helper Text
export const WithHelperText: Story = {
  args: {
    label: 'Email',
    placeholder: 'john@example.com',
    helperText: "We'll never share your email.",
  },
};

// Error
export const ErrorState: Story = {
  args: {
    label: 'Password',
    placeholder: 'Enter password',
    error: true,
    helperText: 'Password must be at least 8 characters.',
  },
};

// Disabled
export const Disabled: Story = {
  args: {
    label: 'API Key',
    placeholder: 'sk-1234567890',
    disabled: true,
    value: 'sk-1234567890',
  },
};

// With Icon
export const WithIcon: Story = {
  render: () => (
    <div className="flex flex-col gap-4 max-w-sm">
      <Input
        placeholder="Search projects..."
        leftIcon={<span>🔍</span>}
      />
      <Input
        placeholder="https://youtube.com/..."
        rightIcon={<span>🔗</span>}
      />
    </div>
  ),
};
