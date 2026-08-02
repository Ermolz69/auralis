import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';
import { Input } from './Input';
import { Icon } from '../icon';

const meta = {
  title: 'Shared UI/Input',
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
    helperText: 'Use at least 8 characters.',
    errorText: 'Password is required.',
  },
};

// Disabled
export const Disabled: Story = {
  args: {
    label: 'API Key',
    placeholder: 'sk-1234567890',
    disabled: true,
    defaultValue: 'sk-1234567890',
  },
};

// With Icon
export const WithIcon: Story = {
  render: () => (
    <div className="flex flex-col gap-4 max-w-sm">
      <Input placeholder="Search projects..." leftIcon={<Icon name="Search" size="sm" />} />
      <Input placeholder="https://youtube.com/..." rightIcon={<Icon name="Link" size="sm" />} />
    </div>
  ),
};

export const LongLabelsAndDescriptions: Story = {
  render: () => (
    <div className="flex max-w-sm flex-col gap-4">
      <Input
        label="Project source display name used by collaborators"
        helperText="Long helper text wraps without changing the control relationship."
        placeholder="Quarterly launch recording"
      />
      <Input
        label="Transcript import URL"
        helperText="Paste a supported source URL."
        errorText="This URL cannot be reached. Check the host and try again."
        error
        defaultValue="https://video.example.com/private/session"
      />
    </div>
  ),
};
