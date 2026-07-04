import type { Meta, StoryObj } from '@storybook/react-vite';
import { Select } from './Select';

const meta = {
  title: 'Shared UI/Select',
  component: Select,
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
} satisfies Meta<typeof Select>;

export default meta;
type Story = StoryObj<typeof meta>;

const defaultOptions = [
  { value: 'apple', label: 'Apple' },
  { value: 'banana', label: 'Banana' },
  { value: 'orange', label: 'Orange' },
];

// Default
export const Default: Story = {
  args: {
    label: 'Select Fruit',
    options: defaultOptions,
  },
};

// With Placeholder
export const WithPlaceholder: Story = {
  args: {
    label: 'Favorite Fruit',
    placeholder: 'Choose a fruit...',
    options: defaultOptions,
  },
};

// Disabled
export const Disabled: Story = {
  args: {
    label: 'Select Fruit',
    placeholder: 'Choose a fruit...',
    options: defaultOptions,
    disabled: true,
  },
};

// Error
export const ErrorState: Story = {
  args: {
    label: 'Select Fruit',
    placeholder: 'Choose a fruit...',
    options: defaultOptions,
    error: true,
    helperText: 'You must select a fruit to continue.',
  },
};

// Many Options
export const ManyOptions: Story = {
  args: {
    label: 'Select Country',
    placeholder: 'Choose a country...',
    options: Array.from({ length: 50 }, (_, i) => ({
      value: `country-${i}`,
      label: `Country ${i + 1}`,
    })),
  },
};
