import { useEffect, type ReactNode } from 'react';
import { LibraryProvider, useLibrary } from '../src/entities/library';
import { ProjectProvider, useProjectContext } from '../src/entities/project';
import { JobContext } from '../src/entities/job';
import { AppUpdateProvider } from '../src/features/app-update';
import { NativeThemeProvider } from '../src/app/providers/NativeThemeProvider';
import {
  emptyLibrary,
  type LibraryAsset,
  type LibraryState,
} from '../src/shared/api/contracts/library';
import type { Project } from '../src/shared/api/contracts/project';
import type { Artifact } from '../src/shared/api/contracts/artifact';
import type { TranslationLink } from '../src/shared/api/contracts/translation';

export const project: Project = {
  id: 'project-one',
  title: 'First project',
  revision: 3,
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '',
  updatedAt: '',
};
export const asset: LibraryAsset = {
  id: 'material-one',
  ownerProjectId: project.id,
  name: 'First text',
  kind: 'text',
  artifactId: null,
  format: 'txt',
  text: 'Original text',
  parentId: null,
  tags: [],
  openedAt: null,
};
export const catalog = {
  profiles: [
    { profile_id: 'native-plan-voxcpm2', profile_sha256: 'a'.repeat(64), engine_id: 'voxcpm2' },
  ],
  voices: [
    {
      voice_id: 'fixture-voice',
      display_name: 'Fixture voice',
      description: '',
      manifest_sha256: 'b'.repeat(64),
      references: [
        {
          reference_id: 'neutral',
          engine_id: 'voxcpm2',
          emotion_id: 'neutral',
          transcript: 'Fixture',
          audio_sha256: 'c'.repeat(64),
          qualification_status: 'qualified',
        },
      ],
    },
  ],
};
export function artifact(id: string, values: Partial<Artifact> = {}): Artifact {
  return {
    id,
    kind: 'generatedSpeech',
    state: 'ready',
    location: { kind: 'storageKey', value: `project/${id}.wav` },
    sizeBytes: 128,
    createdAt: '',
    updatedAt: '',
    readyAt: '',
    ...values,
  };
}
export function translationLink(values: Partial<TranslationLink> = {}): TranslationLink {
  return {
    translationId: 'translation-one',
    projectId: project.id,
    sourceArtifactId: 'source-subtitle',
    sourceSha256: 'a'.repeat(64),
    sourceFormat: 'srt',
    sourceLanguage: 'zh',
    targetLanguage: 'ru',
    activeRunId: null,
    selectedResultId: 'result-one',
    outputArtifactId: 'translated-output',
    revision: 1,
    createdAt: '2026-10-28T00:00:00Z',
    updatedAt: '2026-10-28T00:00:00Z',
    ...values,
  };
}
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
export function backend() {
  const state = emptyLibrary();
  state.assets = [structuredClone(asset)];
  const data = {
    state,
    projects: [structuredClone(project)],
    artifacts: [] as Artifact[],
    translations: [] as TranslationLink[],
    overrides: {} as Record<string, (args: Record<string, any>) => unknown>,
    async handle(command: string, args: Record<string, any> = {}): Promise<any> {
      if (data.overrides[command]) return data.overrides[command](args);
      switch (command) {
        case 'get_library_cmd':
          return structuredClone(data.state);
        case 'list_projects_cmd':
          return structuredClone(data.projects);
        case 'save_library_cmd':
          data.state = structuredClone(args.state);
          data.state.revision++;
          return structuredClone(data.state);
        case 'get_project_cmd':
          return structuredClone(data.projects.find((item) => item.id === args.projectId));
        case 'validate_media_project_cmd':
          return null;
        case 'get_tool_context_cmd':
          return {
            projectId: args.projectId,
            assetId: args.assetId,
            artifactId:
              data.state.assets.find((item) => item.id === args.assetId)?.artifactId ?? null,
            version: 'version-one',
            trackId: null,
          };
        case 'get_media_project_cmd':
          return { results: [] };
        case 'list_project_artifacts_cmd':
          return structuredClone(
            args.kind ? data.artifacts.filter((item) => item.kind === args.kind) : data.artifacts,
          );
        case 'list_project_translations_cmd':
          return structuredClone(data.translations);
        case 'get_speech_catalog_cmd':
          return structuredClone(catalog);
        case 'get_speech_voice_selection_cmd':
          return null;
        case 'get_color_theme_cmd':
          return null;
        case 'get_project_avatar_cmd':
          return { dataUrl: null, initialized: true };
        case 'get_transcript_cmd':
          return null;
        case 'list_youtube_subtitle_tracks_cmd':
        case 'list_speech_runs_cmd':
        case 'list_audio_assemblies_cmd':
          return [];
        case 'resolve_artifact_path_cmd':
          return `C:/fixture/${args.artifactId}.wav`;
        case 'rename_project_cmd': {
          const item = data.projects.find((item) => item.id === args.projectId)!;
          item.title = args.title;
          item.revision++;
          return structuredClone(item);
        }
        case 'create_project_cmd': {
          const item = { ...project, id: `created-${data.projects.length}`, title: args.title };
          data.projects.push(item);
          return structuredClone(item);
        }
        default:
          throw new Error(`Unexpected IPC: ${command}`);
      }
    },
  };
  return data;
}
export function LibraryHarness({ children }: { children: ReactNode }) {
  return (
    <NativeThemeProvider>
      <AppUpdateProvider autoCheck={false}>
        <LibraryProvider>
          {children}
          <LibrarySnapshot />
        </LibraryProvider>
      </AppUpdateProvider>
    </NativeThemeProvider>
  );
}
function LibrarySnapshot() {
  const { state, loading } = useLibrary();
  return (
    <pre aria-label="Library snapshot" data-loaded={!loading}>
      {JSON.stringify(state)}
    </pre>
  );
}
export function ProjectHarness({
  children,
  selected = project,
}: {
  children: ReactNode;
  selected?: Project | null;
}) {
  return (
    <ProjectProvider>
      <ProjectSelection selected={selected} />
      <JobContext.Provider
        value={
          { jobs: {}, phase: 'ready', pendingRefetch: false } as React.ContextType<
            typeof JobContext
          >
        }
      >
        {children}
      </JobContext.Provider>
    </ProjectProvider>
  );
}
function ProjectSelection({ selected }: { selected: Project | null }) {
  const { setProject } = useProjectContext();
  useEffect(() => {
    setProject(selected);
  }, [selected]);
  return null;
}
export function snapshot(element: HTMLElement): LibraryState {
  return JSON.parse(element.textContent!);
}
export function dialogSupport() {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
}
