import { SourceWorkspace } from '@/pages/project';
import { SubtitleTrackPicker } from '@/pages/project';
import { TranscriptResult } from '@/pages/project';
import { useProjectContext } from '@/entities/project';
import { useTranscript, useYoutubeSubtitleTracks } from '@/entities/transcript';
import { useEffect, useState } from 'react';
import { isActiveJobStatus, useProjectJobs } from '@/entities/job';
export function SourceTool() {
  const { projectId } = useProjectContext();
  const tracks = useYoutubeSubtitleTracks(projectId);
  const transcript = useTranscript(projectId);
  const [selected, setSelected] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const { jobs } = useProjectJobs(projectId);
  useEffect(() => {
    setSelected((current) => current ?? tracks.tracks[0]?.id ?? null);
  }, [tracks.tracks]);
  return (
    <div className="space-y-5">
      <SourceWorkspace />
      <SubtitleTrackPicker
        tracks={tracks.tracks.filter((track) =>
          `${track.language} ${track.label ?? ''}`
            .toLocaleLowerCase()
            .includes(query.toLocaleLowerCase()),
        )}
        selectedTrack={tracks.tracks.find((t) => t.id === selected) ?? null}
        selectedTrackId={selected}
        query={query}
        isLoading={tracks.isLoading}
        error={tracks.error}
        isRunning={jobs.some((job) => isActiveJobStatus(job.status))}
        hasTranscript={!!transcript.transcript}
        onQueryChange={setQuery}
        onSelect={setSelected}
        onRefresh={tracks.refresh}
      />
      <TranscriptResult
        transcript={transcript.transcript}
        isLoading={transcript.isLoading}
        error={transcript.error}
        onRefresh={transcript.refetch}
      />
    </div>
  );
}
