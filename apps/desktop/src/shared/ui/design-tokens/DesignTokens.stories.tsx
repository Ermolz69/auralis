import type { Meta, StoryObj } from '@storybook/react-vite';
import React from 'react';

const meta = {
  title: 'Design System/Tokens',
  parameters: {
    layout: 'padded',
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

const colors = [
  { name: 'Background', var: 'var(--color-bg)' },
  { name: 'Surface', var: 'var(--color-surface)' },
  { name: 'Border', var: 'var(--color-border)' },
  { name: 'Text', var: 'var(--color-text)' },
  { name: 'Muted Text', var: 'var(--color-muted)' },
  { name: 'Primary', var: 'var(--color-primary)' },
  { name: 'Accent', var: 'var(--color-accent)' },
  { name: 'Danger', var: 'var(--color-danger)' },
  { name: 'Success', var: 'var(--color-success)' },
  { name: 'Warning', var: 'var(--color-warning)' },
];

const typography = [
  { label: 'Heading 1', classes: 'text-4xl font-bold' },
  { label: 'Heading 2', classes: 'text-3xl font-bold' },
  { label: 'Heading 3', classes: 'text-2xl font-bold' },
  { label: 'Body Large', classes: 'text-lg' },
  { label: 'Body Base', classes: 'text-base' },
  { label: 'Body Small', classes: 'text-sm text-muted' },
  { label: 'Caption', classes: 'text-xs text-muted' },
];

const spacing = [
  { name: '1 (0.25rem)', class: 'w-1' },
  { name: '2 (0.5rem)', class: 'w-2' },
  { name: '4 (1rem)', class: 'w-4' },
  { name: '8 (2rem)', class: 'w-8' },
  { name: '12 (3rem)', class: 'w-12' },
  { name: '16 (4rem)', class: 'w-16' },
];

export const AllTokens: Story = {
  render: () => (
    <div className="flex flex-col gap-12 text-text">
      {/* Colors Section */}
      <section>
        <h2 className="text-2xl font-bold mb-6 border-b border-muted pb-2">Colors & Backgrounds</h2>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-6">
          {colors.map((color) => (
            <div key={color.name} className="flex flex-col gap-2">
              <div
                className="h-24 rounded-lg shadow-sm border border-dashed border-muted flex items-center justify-center relative overflow-hidden"
                style={{ backgroundColor: color.var }}
              >
                <span className="opacity-0 hover:opacity-100 transition-opacity bg-black/50 text-white w-full h-full flex items-center justify-center text-xs font-mono absolute inset-0">
                  {color.var}
                </span>
              </div>
              <div className="flex flex-col">
                <span className="font-semibold text-sm">{color.name}</span>
                <span className="text-xs text-muted font-mono">{color.var}</span>
              </div>
            </div>
          ))}
        </div>
        <p className="text-sm text-muted mt-4">
          * Missing tokens (e.g. border) will appear as transparent blocks with a dashed border.
        </p>
      </section>

      {/* Typography Section */}
      <section>
        <h2 className="text-2xl font-bold mb-6 border-b border-muted pb-2">Typography</h2>
        <div className="flex flex-col gap-6">
          {typography.map((type) => (
            <div
              key={type.label}
              className="flex flex-col md:flex-row md:items-baseline gap-2 md:gap-8 border-b border-surface pb-4 last:border-0"
            >
              <span className="w-32 text-sm text-muted font-mono shrink-0">{type.classes}</span>
              <span className={type.classes}>
                The quick brown fox jumps over the lazy dog ({type.label})
              </span>
            </div>
          ))}
        </div>
      </section>

      {/* Spacing Section */}
      <section>
        <h2 className="text-2xl font-bold mb-6 border-b border-muted pb-2">Spacing</h2>
        <div className="flex flex-col gap-4">
          {spacing.map((space) => (
            <div key={space.name} className="flex items-center gap-4">
              <span className="w-24 text-sm font-mono text-muted">{space.name}</span>
              <div className="bg-surface rounded-md p-1 border border-dashed border-muted">
                <div className={`h-8 bg-primary rounded-sm ${space.class}`} />
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Premium Card Example */}
      <section>
        <h2 className="text-2xl font-bold mb-6 border-b border-muted pb-2">Premium Card Example</h2>

        <div className="bg-surface p-8 rounded-2xl border border-muted shadow-2xl w-full max-w-md relative overflow-hidden group hover:border-primary transition-colors duration-300">
          <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-primary via-accent to-primary" />

          <div className="flex items-start justify-between mb-6">
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-xl bg-primary/20 flex items-center justify-center text-primary border border-primary/30">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
                </svg>
              </div>
              <div>
                <h3 className="text-xl font-bold text-text">Pro Plan</h3>
                <p className="text-sm text-muted">For power users</p>
              </div>
            </div>
            <div className="text-right">
              <div className="text-2xl font-bold text-text">$29</div>
              <div className="text-xs text-muted">/ month</div>
            </div>
          </div>

          <ul className="flex flex-col gap-3 mb-8">
            {['Unlimited projects', 'Advanced analytics', '24/7 Priority support'].map(
              (feature) => (
                <li key={feature} className="flex items-center gap-3 text-sm text-text">
                  <div className="w-5 h-5 rounded-full bg-success/20 flex items-center justify-center text-success">
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
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                  </div>
                  {feature}
                </li>
              ),
            )}
          </ul>

          <button className="w-full py-3 px-4 bg-primary hover:bg-accent text-white rounded-lg font-medium transition-colors duration-200">
            Upgrade to Pro
          </button>
        </div>
      </section>
    </div>
  ),
};
