# Reviewed WAV and video publication

The first video policy replaces the original audio with the approved assembled
Russian speech. It does not retain music or original dialogue. The source video,
raw speech and previous ready outputs remain immutable.

An optional managed source-video artifact becomes part of the immutable assembly
preview and plan. Its bytes, size, duration, streams and media-tool identities
are verified before review and again before work. A changed selection invalidates
the preview. Reject speech longer than the original video; never trim speech or
silently accelerate it. Pad shorter speech with silence to the source duration.

After WAV assembly, a separate owned FFmpeg process encodes H.264/AAC MP4 with
one video and one Russian audio stream. It uses bounded input snapshots, output
budgets, deadlines and stderr, shared machine admission, cancellation and
confirmed process-tree release. Verify the output streams and duration before
publication. Keep codec playback verification separate from linguistic/listening
acceptance.

WAV-only plans retain their previous serialization and two-file publication.
Video plans publish WAV, assembly manifest, MP4 and video manifest as one complete
transaction/outbox-backed result. Pending, cancelled, damaged or partially
finalized results cannot expose playable output. Resume retains the pinned source
and settings; duplicate starts cannot create another active attempt.

Model-free controls cover consent boundaries, old-plan compatibility, source
ownership, stale preview, publication completeness, cancellation and stream/
duration refusal. Browser fixture assertions remain distinct from actual
Windows media processing and playback. Real inference and real media acceptance
require a separately frozen case after resource admission is available.
