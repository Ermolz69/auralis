// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { WorkspaceSection } from './WorkspaceSection';

afterEach(() => cleanup());

describe('WorkspaceSection', () => {
  it('links the section to its localized heading and keeps actions separate', () => {
    render(
      <WorkspaceSection title="Способ получения" action={<button type="button">Изменить</button>}>
        <p>Содержимое секции</p>
      </WorkspaceSection>,
    );

    const heading = screen.getByRole('heading', { name: 'Способ получения' });
    const section = screen.getByRole('region', { name: 'Способ получения' });

    expect(heading.id).toBe('workspace-способ-получения');
    expect(section.getAttribute('aria-labelledby')).toBe(heading.id);
    expect(screen.getByRole('button', { name: 'Изменить' })).not.toBeNull();
    expect(screen.getByText('Содержимое секции')).not.toBeNull();
  });
});
