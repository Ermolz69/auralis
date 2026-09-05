import type { SubtitleTrack } from '@/entities/transcript';
import { RunDubbing } from '@/features/run-dubbing';
import { Button } from '@/shared/ui/button';
import { Icon } from '@/shared/ui/icon';
import { TrackList } from './TrackList';
import { WorkspaceSection } from '../WorkspaceSection';

type Props = {
  tracks: SubtitleTrack[];
  selectedTrack: SubtitleTrack | null;
  selectedTrackId: string | null;
  query: string;
  isLoading: boolean;
  error: string | null;
  isRunning: boolean;
  hasTranscript: boolean;
  onQueryChange: (value: string) => void;
  onSelect: (id: string) => void;
  onRefresh: () => void;
};

export function SubtitleTrackPicker(props: Props) {
  return (
    <WorkspaceSection
      title="Доступные дорожки"
      action={
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void props.onRefresh()}
          disabled={props.isLoading}
        >
          {props.isLoading ? 'Обновление…' : 'Обновить'}
        </Button>
      }
    >
      <div className="relative mb-2">
        <Icon
          name="Search"
          size={13}
          color="muted"
          className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2"
        />
        <input
          type="search"
          value={props.query}
          onChange={(event) => props.onQueryChange(event.target.value)}
          placeholder="Поиск по языку или типу дорожки…"
          aria-label="Поиск по доступным дорожкам"
          className="h-9 w-full rounded-md border border-border bg-surface-raised pl-8 pr-3 text-xs text-text outline-none placeholder:text-subtle focus:border-primary focus:ring-2 focus:ring-primary/20"
        />
      </div>
      <TrackList
        tracks={props.tracks}
        selectedTrackId={props.selectedTrackId}
        onSelect={props.onSelect}
        isLoading={props.isLoading}
        error={props.error}
        hasQuery={Boolean(props.query.trim())}
      />
      <div className="mt-3 flex justify-end">
        <RunDubbing
          subtitleTrack={props.selectedTrack}
          label={props.hasTranscript ? 'Получить заново' : 'Получить субтитры'}
          disabled={!props.selectedTrack || props.isLoading || props.isRunning}
        />
      </div>
    </WorkspaceSection>
  );
}
