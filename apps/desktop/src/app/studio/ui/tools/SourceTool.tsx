import { SourceWorkspace } from '@/pages/project';
import { SubtitleTrackPicker } from '@/pages/project';
import { TranscriptResult } from '@/pages/project';
import { useProjectContext } from '@/entities/project';
import { useTranscript, useYoutubeSubtitleTracks } from '@/entities/transcript';
import { useEffect, useState } from 'react';
import { isActiveJobStatus, useProjectJobs } from '@/entities/job';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import { invoke } from '@/shared/api/tauri';
import { convertFileSrc } from '@tauri-apps/api/core';
export function SourceTool({ asset }: { asset?: LibraryAsset }) {
  const [media, setMedia] = useState('');
  const [mediaError, setMediaError] = useState('');
  useEffect(() => {
    let active = true;
    setMedia('');
    setMediaError('');
    if (asset?.artifactId)
      void invoke('resolve_artifact_path_cmd', { artifactId: asset.artifactId })
        .then((path) => {
          if (active) setMedia(convertFileSrc(path));
        })
        .catch((e) => {
          if (active) setMediaError(String(e));
        });
    return () => {
      active = false;
    };
  }, [asset?.artifactId]);
  const { projectId, project } = useProjectContext();
  const ownsSource =
    !asset?.artifactId ||
    (project?.source?.kind === 'managedLocalFile' &&
      project.source.artifactId === asset.artifactId);
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
      {asset?.artifactId ? (
        <section className="space-y-3">
          <h2 className="text-xl font-semibold">{asset.name}</h2>
          {mediaError && <p role="alert">{mediaError}</p>}
          {media &&
            (asset.kind === 'audio' ? (
              <audio controls src={media} aria-label={asset.name} />
            ) : (
              <video
                controls
                preload="metadata"
                src={media}
                aria-label={asset.name}
                className="max-h-[50vh] w-full bg-canvas"
              />
            ))}
        </section>
      ) : (
        <SourceWorkspace />
      )}
      {ownsSource && (
        <>
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
        </>
      )}
    </div>
  );
}
