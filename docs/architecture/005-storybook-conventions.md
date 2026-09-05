# Storybook Conventions

Use these conventions when adding components and state catalogs to Storybook.

## 1. Mandatory Stories

- Every public visual component in `shared/ui` must have an accompanying story.
- Helper-only modules may be exercised through the stories and tests of the fields or components that render them.
- Pages, widgets, and user-facing features should add state-focused stories when isolated rendering is useful.

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
- Use `Shared UI/[Component Name]` for shared components.
- Use `Features/...`, `Widgets/...`, and `Pages/...` for higher FSD layers.
- Example: `title: 'Shared UI/Button'`

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
import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from './Button';

const meta = {
  title: 'Shared UI/Button',
  component: Button,
  tags: ['autodocs'],
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;

// 1. Default/Primary usage
export const Primary: Story = {
  args: { variant: 'primary', children: 'Click Me' },
};

// 2. State: Disabled
export const Disabled: Story = {
  args: { variant: 'primary', children: 'Not allowed', disabled: true },
};

// 3. State: Loading
export const Loading: Story = {
  args: { variant: 'primary', children: 'Submitting...', loading: true },
};
```

`task check:frontend` executes unit tests and Storybook browser tests in Chromium.
Use `task frontend:storybook` when a standalone static Storybook build also needs
verification.
