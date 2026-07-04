# Frontend Architecture (FSD)

The frontend application uses **React + Vite** and strictly adheres to the **Feature-Sliced Design (FSD)** methodology.

## FSD Layers & Responsibilities

- **app**: Global application setup, context providers, router configuration, and global CSS.
- **pages**: Top-level route components. Pages act as layout orchestrators and must rely purely on composition.
- **widgets**: Complex, independent UI blocks that compose features and entities (e.g., `ProjectHeader`, `JobQueuePanel`).
- **features**: Slices containing specific user scenarios or interactive actions (e.g., `PasteYoutubeLink`, `RunDubbing`).
- **entities**: UI representation and state of business concepts (e.g., `Project`, `Transcript`).
- **shared**: Reusable, agnostic primitives including the UI kit, generic hooks, helpers, types, and API configuration.

## Boundaries and Imports

- **Public API**: Every slice (e.g., inside `widgets`, `features`, `entities`) must expose its external interface via an `index.ts` file.
- **No Deep Imports**: Importing internal paths of a slice from the outside is strictly prohibited. You must import exclusively from the slice's `index.ts` public API.
- **Layer Directionality**: Modules can only import from layers below them:
  `app` -> `pages` -> `widgets` -> `features` -> `entities` -> `shared`

## Rules for Pages

Pages must remain as thin as possible. They are strictly prohibited from containing:
- Business logic or complex state management.
- Direct external API calls (these belong in features or entities).
- Raw UI components (if they can be abstracted into widgets or features).

### Correct Page Composition Example

```tsx
import { ProjectHeader } from '@/widgets/project-header';
import { TranscriptEditor } from '@/widgets/transcript-editor';
import { JobQueuePanel } from '@/widgets/job-queue-panel';
import { ExportPanel } from '@/widgets/export-panel';

export const ProjectPage = () => {
  return (
    <div className="h-screen flex flex-col bg-bg text-text font-sans">
      <ProjectHeader />
      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex flex-col min-w-0">
          <TranscriptEditor />
          <ExportPanel />
        </div>
        <JobQueuePanel />
      </div>
    </div>
  );
};
```
