import { useEffect, useMemo, useState } from 'react';
import { useProjectJobs } from '@/entities/job';
import { useProjectContext } from '@/entities/project';
import { useTranscript, useYoutubeSubtitleTracks } from '@/entities/transcript';
import { filterTracks } from './subtitle-workspace/model';
import { SubtitleActivityLog } from './subtitle-workspace/SubtitleActivityLog';
import { SubtitleMethods } from './subtitle-workspace/SubtitleMethods';
import { SubtitleTrackPicker } from './subtitle-workspace/SubtitleTrackPicker';
import { TranscriptResult } from './subtitle-workspace/TranscriptResult';

export function SubtitleWorkspace() {
  const { project } = useProjectContext();
  const { jobs } = useProjectJobs(project?.id ?? null);
  const transcriptState = useTranscript(project?.id ?? null);
  const trackState = useYoutubeSubtitleTracks(project?.id ?? null);
  const [selectedTrackId, setSelectedTrackId] = useState<string | null>(null);
  const [trackQuery, setTrackQuery] = useState('');

  useEffect(() => {
    setSelectedTrackId((current) => {
      if (current && trackState.tracks.some((track) => track.id === current)) return current;
      return trackState.tracks[0]?.id ?? null;
    });
  }, [trackState.tracks]);

  const activity = useMemo(
    () => [...jobs].sort((left, right) => left.updatedAt.localeCompare(right.updatedAt)),
    [jobs],
  );
  const selectedTrack = trackState.tracks.find((track) => track.id === selectedTrackId) ?? null;

  return (
    <section
      className="h-full min-h-0 overflow-y-auto px-4 py-5 sm:px-6"
      aria-label="Subtitle source configuration"
      data-testid="subtitle-workspace"
    >
      <div className="space-y-5">
        <SubtitleMethods />
        <SubtitleTrackPicker
          tracks={filterTracks(trackState.tracks, trackQuery)}
          selectedTrack={selectedTrack}
          selectedTrackId={selectedTrackId}
          query={trackQuery}
          isLoading={trackState.isLoading}
          error={trackState.error}
          isRunning={activity.some((job) => job.status === 'pending' || job.status === 'running')}
          hasTranscript={Boolean(transcriptState.transcript)}
          onQueryChange={setTrackQuery}
          onSelect={setSelectedTrackId}
          onRefresh={trackState.refresh}
        />
        {(transcriptState.isLoading || transcriptState.error || transcriptState.transcript) && (
          <TranscriptResult
            transcript={transcriptState.transcript}
            isLoading={transcriptState.isLoading}
            error={transcriptState.error}
            onRefresh={transcriptState.refetch}
          />
        )}
        <SubtitleActivityLog jobs={activity} />
      </div>
    </section>
  );
}
