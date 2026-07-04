import type { Meta, StoryObj } from '@storybook/react';
import { Textarea } from './Textarea';

const meta = {
  title: 'UI Kit/Textarea',
  component: Textarea,
  parameters: {
    layout: 'padded',
  },
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    helperText: { control: 'text' },
    error: { control: 'boolean' },
    disabled: { control: 'boolean' },
    resizable: { control: 'boolean' },
  },
} satisfies Meta<typeof Textarea>;

export default meta;
type Story = StoryObj<typeof meta>;

// Default
export const Default: Story = {
  args: { placeholder: 'Enter your message...' },
};

// With Label
export const WithLabel: Story = {
  args: { label: 'Description', placeholder: 'Describe your project' },
};

// Error
export const ErrorState: Story = {
  args: {
    label: 'Bio',
    defaultValue: 'This bio is way too short.',
    error: true,
    helperText: 'Bio must be at least 100 characters.',
  },
};

// Disabled
export const Disabled: Story = {
  args: {
    label: 'Read-only notes',
    disabled: true,
    value: 'These notes cannot be edited anymore.',
  },
};

// Long Text
export const LongText: Story = {
  args: {
    label: 'Terms of Service',
    value:
      'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.',
    rows: 6,
  },
};
