// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { HomePage } from './HomePage';

vi.mock('../../../features/import-local-media', () => ({
  ImportLocalMediaButton: () => <button type="button">Import local video</button>,
}));

vi.mock('../../../features/paste-youtube-link', () => ({
  PasteYoutubeLink: () => <button type="button">Add from YouTube</button>,
}));

vi.mock('../../../features/project-list', () => ({
  ProjectList: () => <section aria-label="Recent Projects" />,
}));

describe('HomePage', () => {
  it('presents local import as the first primary creation path', () => {
    render(<HomePage />);

    const createProject = screen.getByLabelText('Create project');
    const localImport = screen.getByRole('button', { name: 'Import local video' });
    const youtube = screen.getByRole('button', { name: 'Add from YouTube' });

    expect(createProject.compareDocumentPosition(localImport)).toBe(
      Node.DOCUMENT_POSITION_CONTAINED_BY | Node.DOCUMENT_POSITION_FOLLOWING,
    );
    expect(
      localImport.compareDocumentPosition(youtube) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });
});
