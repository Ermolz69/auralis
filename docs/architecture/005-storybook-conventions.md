# Storybook Conventions

To prevent Storybook from becoming a chaotic dump of random demos, all developers must adhere to the following conventions when adding components to the UI Kit.

## 1. Mandatory Stories

- **Every shared UI component must have a story**. If a component lives in `shared/ui`, it must have an accompanying `.stories.tsx` file.
- Stories are the primary documentation for the design system. If it's not in Storybook, it doesn't exist.

## 2. File Placement

- Stories must live **right next to the component** they document.
- Example:

  ```text
  shared/ui/button/
    ├── Button.tsx
    └── Button.stories.tsx
  ```

## 3. Naming Strategy

- Titles in Storybook should follow a strict hierarchy to keep the sidebar organized.
- Use `UI Kit / [Component Name]` for standard components.
- Example: `title: 'UI Kit/Button'`

## 4. Variants and States Coverage

Every interactive component must show all its possible visual variations and states in the story. Do not hide states behind interactive controls only; explicitly render them so visual regression tools (or developers) can see them at a glance.

- **Variants**: If a component has `primary`, `secondary`, and `danger` variants, all three must be exported as separate stories or combined into a "All Variants" grid story.
- **States**: Interactive components (Buttons, Inputs, etc.) must explicitly demonstrate their specific states:
  - `Default`
  - `Disabled`
  - `Loading` (if applicable)
  - `Error` (if applicable)

## 5. Story Template Example

```tsx
import type { Meta, StoryObj } from '@storybook/react';
import { Button } from './Button';

const meta = {
  title: 'UI Kit/Button',
  component: Button,
  tags: ['autodocs'],
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;

// 1. Default/Primary usage
export const Primary: Story = {
  args: { variant: 'primary', label: 'Click Me' },
};

// 2. State: Disabled
export const Disabled: Story = {
  args: { variant: 'primary', label: 'Not allowed', disabled: true },
};

// 3. State: Loading
export const Loading: Story = {
  args: { variant: 'primary', label: 'Submitting...', loading: true },
};
```
