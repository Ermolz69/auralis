# Frontend Architecture (FSD)

The frontend uses React, Vite, TypeScript, and Feature-Sliced Design (FSD).

## FSD Layers & Responsibilities

- **app**: Global application setup, providers, routing composition, and global CSS.
- **pages**: Top-level route and workspace composition. Pages may coordinate view-local state but do not own backend or domain rules.
- **widgets**: Complex, independent UI blocks that compose features and entities (e.g., `ProjectHeader`, `JobQueuePanel`).
- **features**: Slices containing specific user scenarios or interactive actions (e.g., `PasteYoutubeLink`, `RunDubbing`).
- **entities**: UI representation and state of business concepts (e.g., `Project`, `Transcript`).
- **shared**: Reusable primitives, API contracts and Tauri transport, routing, themes, helpers, types, and the UI kit.

## Boundaries and Imports

- **Public API**: Cross-slice imports use the target slice's `index.ts`. Relative imports are used inside a slice.
- **No Deep Imports**: External access to another slice's internal files is prohibited.
- **Layer Directionality**: Modules can only import from layers below them:
  `app` -> `pages` -> `widgets` -> `features` -> `entities` -> `shared`

`apps/desktop/eslint.config.mjs` enforces layer direction, public APIs, and the
central icon registry. `task q:fsd-boundaries` runs configuration regressions,
ESLint, and the additional import scan.

## Rules for Pages

Pages remain focused on route-level composition. They must not contain:

- Domain state transitions or backend orchestration.
- Direct Tauri calls; transport access belongs in entity or feature APIs.
- Reusable behavior that belongs in a feature, entity, widget, or `shared` module.

Pages may own route-local selection, loading, and layout state and may compose
shared UI directly when the element is specific to that page.

### Page Composition Example

```tsx
import { ProjectHeader } from '@/widgets/project-header';
import { TranscriptEditor } from '@/widgets/transcript-editor';
import { MediaPanel } from '@/widgets/media-panel';

export const ProjectPage = () => {
  return (
    <main>
      <ProjectHeader />
      <MediaPanel />
      <TranscriptEditor />
    </main>
  );
};
```
