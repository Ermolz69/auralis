// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './Tabs';

afterEach(() => cleanup());

describe('Tabs', () => {
  it('links tabs and panels with accessible relationships', () => {
    render(
      <Tabs defaultValue="general">
        <TabsList fullWidth>
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="advanced">Advanced</TabsTrigger>
        </TabsList>
        <TabsContent value="general">General content</TabsContent>
        <TabsContent value="advanced">Advanced content</TabsContent>
      </Tabs>,
    );

    const tab = screen.getByRole('tab', { name: 'General' });
    const panel = screen.getByRole('tabpanel');

    expect(tab.id).not.toBe('');
    expect(panel.id).not.toBe('');
    expect(tab.getAttribute('aria-controls')).toBe(panel.id);
    expect(panel.getAttribute('aria-labelledby')).toBe(tab.id);
    expect(screen.getByRole('tablist').className).toContain('w-full');
    expect(screen.getByRole('tablist').hasAttribute('fullWidth')).toBe(false);
  });
});
