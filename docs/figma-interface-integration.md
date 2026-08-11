# Figma interface integration: analysis and plan

## Stage 1 — analysis

### Main application (`apps/desktop`)

- **Framework and structure:** React 19 + TypeScript 6 + Vite 8 inside a Tauri 2 desktop shell. The frontend follows Feature-Sliced Design: `app`, `pages`, `widgets`, `features`, `entities`, `shared`.
- **Entry points:** `src/main.tsx` mounts providers and `App`; `src/App.tsx` selects the current page; Tauri commands are called through the typed wrapper in `shared/api/tauri`.
- **Routes/pages:** in-memory navigation with `View = 'home' | 'project' | 'settings'`. There is no URL router. `App` prevents opening the project view without a selected project.
- **Existing UI:** reusable Button, Badge, Card, Dialog, Icon, Input, Select, Tabs, Textarea, Progress, Toast and PageLayout primitives. Storybook stories cover the shared system and main screens.
- **Design system:** Tailwind CSS 4 tokens in `app/styles/theme.css`, global rules in `app/styles/index.css`. Feature/page code is expected to use semantic tokens rather than raw colors.
- **Hooks and providers:** `NavigationProvider`, `ProjectProvider`, `AppJobProvider`/`JobProvider`; `useNavigation`, `useProjectContext`, `useJobContext`, `useTranscript`, `useImportLocalMedia`, `usePasteYoutubeLink`.
- **State management:** React context plus local state. Project selection and operation invalidation live in `ProjectProvider`; job synchronization uses a reducer and `JobStoreSynchronizer`. There is no Redux/Zustand store and no persisted UI preference store.
- **API/services:** typed Tauri `invoke`/`listen`; project create/list/delete/get, local-media probe/import, YouTube project creation, mock subtitle pipeline start/retry, job list/snapshot/events/cancel, transcript fetch/events.
- **Models/types:** typed contracts for Project, MediaSource/MediaMetadata/streams/audio tracks, Job/status/progress/events, Transcript/segments, and normalized command errors.
- **Forms/validation:** local controlled inputs and typed validation. YouTube requires a non-empty URL in the UI and relies on backend validation; file import uses the Tauri picker and backend probing. There is no form library or schema library.
- **Loading/error/empty states:** implemented for project listing, import, YouTube creation, transcript fetch, job synchronization, empty job queue, missing media metadata, failed/cancelled/completed jobs, and disabled export/settings.
- **Authentication/permissions:** no authentication, user roles, or access-control layer exists in the frontend contract.
- **Checks:** `pnpm typecheck`, `pnpm lint`, `pnpm test`, `pnpm build`; unit tests use Vitest/jsdom, Storybook browser tests use Playwright.

### Figma export (`figma-export`)

- **Screens:** projects list, workspace with six pipeline steps, settings, collapsible job queue, global sidebar/top bar/status bar.
- **Visual components:** 240 px sidebar; compact 40 px top bar and 28 px status bar; pipeline navigation; step headers; project rows; job cards; metadata tables; radio-card selectors; logs; progress bars; compact controls.
- **Interactive elements:** project switching/creation/rename/pin/delete/open-folder, pipeline navigation, run/retry/cancel, subtitle method/track selection, voice selection, render mode/volume, settings save/reset, collapsible sections, queue drawer and navigation history.
- **Variants/states:** idle, pending, running, completed, error, cancelled; active/selected/locked; loading spinners; empty queue/project list; hover/focus/disabled; determinate and indeterminate progress.
- **Responsive behavior:** no responsive breakpoints are implemented in the export. The fixed sidebar, fixed queue and two-column controls overflow at narrow widths. Mobile/tablet behavior therefore cannot be copied and must be an accessibility-preserving adaptation of the same visual language.
- **Tokens:** Graphite/Ice/Signal Mint palette; matte layered surfaces; 1 px borders; Inter/system sans and JetBrains Mono/system mono; 11–26 px type scale; 4 px spacing base; 4/6/9/12/16 px radii; restrained shadows; mint glow only for focus/active/progress; short 75–300 ms transitions.
- **Hardcoded data:** all four projects, current project/media metadata, six task states, logs, subtitle tracks/previews, translation paths/progress, TTS roles/voices, render duration/volumes/output, job queue, settings values, version and local status.
- **Prototype-only behavior:** timers simulate jobs; rename/pin/delete mutate component state; folder buttons are empty; settings do not persist; source file drop does not pass a real file; translation/TTS/voice/render do not call a backend.

## Mapping table

| Figma screen or component | Main project location | Existing logic/hook/API | Required action |
| --- | --- | --- | --- |
| Global sidebar and app chrome | `widgets/app-shell/ui/AppShell.tsx` | `useNavigation`, `useProjectContext`, `useJobContext` | Rebuild the shell with Figma composition and semantic tokens; preserve the three existing views and focus management. |
| Projects screen | `pages/home/ui/HomePage.tsx`, `features/project-list` | `listProjects`, project events, project context | Restyle as the Figma project surface while keeping the real list and selection behavior. |
| “New project” action | `features/import-local-media`, `features/paste-youtube-link` | existing import hooks and project APIs | Present both real creation paths in a compact Figma-like creation panel; do not add name-only mock creation. |
| Project row | `ProjectListRow.tsx` | project formatters/context | Match the compact row layout; retain safe path formatting, status, open and delete actions. |
| Context menu actions | `DeleteProjectDialog.tsx`, project APIs | only open and delete exist | Keep real open/delete actions. Do not add rename, pin or open-folder controls because no contracts exist. |
| Workspace header/step header | `ProjectHeader.tsx`, `CurrentStepSummary.tsx` | project context, media formatters, job context, `RunDubbing` | Apply compact step header/progress styling and keep real subtitle-import action/state. |
| Source step | home creation UI + `MediaPanel.tsx` | local import and YouTube hooks, media metadata API/contracts | Use the source inputs for creation and show actual source metadata in the workspace. No fake drag/drop. |
| Subtitles step | `TranscriptEditor.tsx`, `TranscriptPanelView.tsx`, `RunDubbing.tsx` | `useTranscript`, transcript events, mock subtitle pipeline command | Present actual transcript/loading/error/empty states and the supported subtitle import action. |
| Translation step | no frontend/backend command | none | Do not implement or enable. May be shown only as unavailable pipeline context. |
| TTS preparation step | no frontend/backend command | none | Do not implement or enable. |
| Voice synthesis step | no frontend/backend command | none | Do not implement voice selection or fake output. |
| Render step/export | `ExportPanel.tsx` | no export API | Retain an explicit unavailable state; do not add working-looking mode/volume/export controls. |
| Job queue/drawer | `JobQueuePanel.tsx`, `JobCard.tsx` | `useJobContext`, job events/snapshot, `cancelJob`, supported retry | Restyle and expose through the shell/workspace using live jobs only. |
| Media details | `MediaPanel.tsx`, audio/stream subcomponents | project metadata | Restyle tables/cards with real metadata and warnings. |
| Status bar | new AppShell UI component | project context, metadata, live jobs | Add a visual status bar derived from existing project/job state; no hardcoded job/version data. |
| Settings | `pages/settings/ui/SettingsPage.tsx` | no settings service/store | Match the Figma structure visually while keeping controls non-interactive and explicitly unavailable. |
| UI primitives/tokens | `shared/ui/*`, `app/styles/*` | existing shared design system | Map Figma colors, typography, radii, shadows and interaction states into semantic tokens and primitives. |
| Responsive behavior | AppShell, HomePage, ProjectPage, secondary panels | existing responsive panel/dialog behavior | Desktop: sidebar/top/status chrome. Tablet/mobile: compact top/bottom navigation, no fixed sidebar, dialogs for secondary panels, stacked forms and safe overflow. |

## Reuse and gaps

### Reuse without business changes

- All typed Tauri APIs, error normalization and event listeners.
- Project, media, job and transcript contracts/formatters.
- Project/Job/Navigation providers and their synchronization rules.
- Local media and YouTube creation hooks.
- Subtitle import/retry/cancel behavior.
- Project delete workflow, focus restoration and toasts.
- Transcript, queue and metadata state branches.
- Shared accessible Dialog, Tabs, Input, Select, Progress and Icon behavior.

### Visual redesign required

- AppShell, HomePage, ProjectPage/secondary panels and SettingsPage.
- Project list/rows, project header/current step, transcript, media, job queue/cards and export unavailable panel.
- Shared tokens and the visual classes of core UI primitives.

### New UI components

- Figma-like responsive application sidebar/top bar/status bar inside the app-shell widget.
- Compact creation surface that composes the existing local and YouTube features.
- A typed presentation adapter for mapping real project/job state to pipeline/status indicators if needed; it must not create new domain states.

### Cannot be connected unambiguously

- Project rename, pinning, project-folder browsing and a default project directory.
- Browser-like forward/back history beyond the existing three-view navigation.
- SRT file import and STT selection.
- Translation, TTS preparation, voice casting/synthesis, final render, output-folder actions and volume settings.
- Persistent appearance, export defaults, logging level/directory and developer settings.

### Conflicts between prototype and application behavior

- Figma treats all six pipeline stages as runnable; the current contract supports media import and a subtitle-import mock pipeline only.
- Figma supports local-file subtitle/STT flows; the application explicitly reports local automatic transcription as unavailable.
- Figma settings look editable and saveable; the application has no settings persistence contract.
- Figma project rename/pin/open-folder controls have no APIs.
- Figma uses local simulated timers and fabricated outputs; production state is event/snapshot-driven and must remain so.
- Figma has no responsive layout; the application already has compact secondary-panel dialogs that must be preserved and restyled.

## Baseline verification

- TypeScript: passed.
- Lint: passed with one pre-existing Fast Refresh warning in `JobProvider.tsx`.
- Unit tests: 36 files / 168 tests passed.
- Production build: passed with the pre-existing large-chunk warning.

## Stage 2 — implementation plan

1. **Tokens and primitives**
   - Update `app/styles/theme.css` and `app/styles/index.css` with the Figma semantic palette, compact typography, radii, focus/progress animation and system font fallbacks.
   - Restyle `shared/ui/button/Button.tsx`, `card/Card.tsx`, `badge/Badge.tsx`, `input/Input.tsx`, `progress/Progress.tsx`, `dialog/Dialog.tsx` and `page-layout/PageLayout.tsx` without changing their public props.
2. **Application chrome**
   - Rework `widgets/app-shell/ui/AppShell.tsx` into a responsive desktop workstation shell connected to navigation, selected project and job state.
   - Add small app-shell presentation components only where separation improves readability; they receive typed props and contain no data fetching.
3. **Projects and real creation flows**
   - Recompose `pages/home/ui/HomePage.tsx` around a projects header, creation panel and real project list.
   - Restyle `features/project-list/ui/ProjectList*.tsx` and its loading/error/empty states while preserving list, open, delete and focus behavior.
   - Restyle existing local/YouTube creation components; retain their hooks and error behavior.
4. **Workspace**
   - Restyle `ProjectHeader.tsx`, `CurrentStepSummary.tsx`, `WorkspaceMain.tsx`, `WorkspaceSecondaryPanels.tsx`, transcript, media, queue and export widgets.
   - Keep the real transcript/job/media sources and responsive secondary-panel dialogs. Unsupported later pipeline steps remain unavailable.
5. **Settings**
   - Restyle `SettingsPage.tsx` as a compact workstation page; keep it informational because no persistence service exists.
6. **Verification**
   - Run TypeScript, lint, unit tests and production build.
   - Start the frontend/Storybook surfaces, compare desktop/tablet/mobile screenshots with the Figma export, correct visible spacing/overflow/focus problems, and rerun affected checks.
