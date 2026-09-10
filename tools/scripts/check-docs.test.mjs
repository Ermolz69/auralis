import assert from 'node:assert/strict';
import test from 'node:test';
import { collectDocumentationErrors } from './check-docs.mjs';

const files = (overrides = []) =>
  new Map([
    ['README.md', '[Documentation](docs/README.md)'],
    ['docs/README.md', '[Architecture](./architecture/overview.md)'],
    ['docs/architecture/overview.md', '# Architecture'],
    ...overrides,
  ]);

test('accepts mandatory files and valid relative or external links', () => {
  const markdownFiles = files([['docs/external.md', '[Storybook](https://storybook.js.org/)']]);
  assert.deepEqual(
    collectDocumentationErrors({
      markdownFiles,
      requiredDocs: ['README.md', 'docs/README.md', 'docs/architecture/overview.md'],
    }),
    [],
  );
});

test('reports every missing mandatory document', () => {
  const errors = collectDocumentationErrors({
    markdownFiles: files(),
    requiredDocs: ['README.md', 'docs/missing.md'],
  });
  assert.deepEqual(errors, ['missing mandatory documentation file: docs/missing.md']);
});

test('rejects broken relative links while allowing local anchors', () => {
  const markdownFiles = files([
    ['docs/README.md', '[Missing](./missing.md) and [Section](#section)'],
  ]);
  const errors = collectDocumentationErrors({ markdownFiles, requiredDocs: [] });
  assert.deepEqual(errors, ['docs/README.md: broken relative Markdown link: ./missing.md']);
});

test('resolves parent paths and encoded filenames', () => {
  const markdownFiles = files([
    ['docs/guide/start.md', '[Root](../../README.md) and [Name](../My%20Guide.md)'],
    ['docs/My Guide.md', '# Guide'],
  ]);
  assert.deepEqual(collectDocumentationErrors({ markdownFiles, requiredDocs: [] }), []);
});

test('reports invalid percent encoding without aborting the scan', () => {
  const markdownFiles = files([['docs/README.md', '[Broken](./bad%name.md)']]);
  assert.deepEqual(collectDocumentationErrors({ markdownFiles, requiredDocs: [] }), [
    'docs/README.md: invalid encoded Markdown link: ./bad%name.md',
  ]);
});
