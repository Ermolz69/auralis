# Three-input revoicing contract

Version: 1, frozen prospectively on 23 October 2026 for implementation. This
extends the subtitle-only integration design under the owner's explicit Goal.
It is not a statement that TXT, ASR or media assembly are already implemented.

## Invariants shared by all inputs

Auralis owns source admission, project revisions, transcript editing, jobs,
admission and media export. Translate owns translation checkpoints, immutable
results, edits and history. CONTRACT owns speech-plan semantics; TTS owns raw
speech/checkpoints and verified synthesis cache. Caster provides frozen data,
without WPF changes. All inference is sequential under shared machine admission.

Original imported bytes remain immutable and available for the project lifetime.
An edit creates a revision with ancestry, stable surviving segment IDs and an
explicit mapping for split/merge/deletion. Ordered source coverage is verified.
Text is data, including markup-looking strings. Never manufacture subtitle
timestamps or a renderable Marker publication to route a different source kind.

Review identifies the exact source revision, selected translation result/digest,
spoken-text revision and decision ID. A plan additionally freezes assignments,
voice/reference/report hashes, profile/model/seed and timing policy. Changed
effective text/cast/settings require a new plan; prior approved work survives.
Preparation of numbers, abbreviations or pronunciation is a visible diff followed
by approval of the exact prepared text. No silent rewrite or loss of meaning.

## Chinese subtitle input

Keep the declared plain-SRT and strict plain-WebVTT grammars, rejection rules,
protected bytes, cue identity/order and original source windows. YouTube import
records video ID/revision, selected track language/kind and retrieved bytes/hash.
Do not relabel automatic captions as manual captions or silently change tracks.
All existing G1–G9 guarantees and unsupported-syntax failures continue to apply.

## Chinese TXT and pasted text: INT-IN01/02

Admit strict UTF-8 (optional BOM), reject invalid encoding and unsupported binary
input before inference. Preserve original newline and paragraph bytes in the
source artifact. Persist a source snapshot and an ordered paragraph table;
paragraph identity is stored, never recomputed from the current ordinal or text.
Editing one paragraph retains its ID; insertions get new IDs. Split/merge records
parent IDs and invalidates affected translation/approval/cache identities.

The translator consumes ordered paragraph text through its format-independent
core. A text adapter preserves paragraph mapping and separately renders Russian
text. It does not pass TXT to the SRT parser or invent timing. Context is Chinese
source only plus explicitly admitted terms. Missing/duplicate translated
paragraphs reject complete publication. G1/G2 apply to paragraph mapping; G3–G9
retain their thresholds with a separately frozen eligible paragraph assessment.

Output mode is continuous narration: ordered speech plus explicit paragraph
pauses. No video is invented for a text-only source. Short and long TXT acceptance
requires the real installed UI, editing/history/review, ready WAV, playback,
cancel/restart/resume and preserved original bytes.

## Subtitle-free video and Chinese ASR: INT-IN03/04 / ASR-01

Implement a separate owned ASR adapter. It receives managed extracted audio and
a pinned model/profile, never source subtitles or a Russian expected transcript.
Persist raw recognition, time units, language-selection method, model/runtime
hashes, confidence when actually available, and an ordered transcript revision.
The editor can correct text and timing before translation; recording the edited
revision does not relabel raw ASR as accurate. Explicit transcript confirmation
and later Russian speech approval are separate decisions.

Before the first accuracy run, freeze actual short and long media and trustworthy
Chinese timed ground truth, normalization, eligibility and numerical thresholds.
The initial candidate is the already researched faster-whisper CPU/int8 boundary;
the old base model is a **candidate only**, not a quality-qualified choice. Pin
actual available artifacts before selection; retain a failed model rather than
silently choosing another after seeing the test answers.

Prospective ASR acceptance: character error rate <=10% across all eligible speech,
zero unresolved critical name/number/negation errors in the predeclared controls,
at least 95% of independently matched segment boundaries within 500 ms, no
reversed/negative/out-of-media times, and explicit complete speech coverage.
Report substitutions/deletions/insertions, denominator, matching algorithm and
unmatched segments as failures. Freeze Unicode/punctuation/script normalization
before inference; do not normalize away lexical errors. A genuine ground truth
is required; old Whisper output is not ground truth. If the candidate fails,
record a new prospective comparison before changing model or thresholds.

ASR short and long cases proceed through transcript correction, translation,
versioned approval, casting, raw synthesis, timed assembly, ready WAV/separate
video, playback and recovery. No ASR worker or model is needed for subtitle/TXT
imports. Missing ASR assets show an actionable readiness state.

## Continuous and timed audio

Continuous narration concatenates complete raw speech with declared pauses;
subtitle mode uses the original source windows. Retain raw synthesis separately
from processed audio and use separate cache identities. Timing changes do not
invalidate unchanged raw speech. Effective text/reference/model/profile/seed
changes do invalidate the affected raw cache entries; corrupt bytes regenerate.

Default policy is preserve full speech at 1.0 tempo, insert silence where needed,
replace the original audio track and discard its speech/background in the new
export. The original media remains available. Separate background/music stems
may be mixed only when supplied and explicitly selected; no source-separation
quality or original-speech removal is assumed.

Optional pitch-preserving acceleration requires an explicit per-plan policy
with a displayed maximum, initially 1.15x. This ceiling is a prospective bound
to be listened to, not a sound-quality approval. Never clip speech or silently
exceed the selected bound. Overlapping source cues, unsupported speaker overlaps
and impossible windows produce visible conflicts and prevent ready timed export.
Preserve exact approved text; resolving a conflict through text editing creates
a new revision and approval. A continuous WAV may still be exported independently
with its mode identified; it is not a synchronized video result.

Managed FFmpeg produces a complete staged WAV and a separate MP4 with copied
compatible video and Russian audio (AAC for MP4); incompatible source video
requires a declared transcode. Auralis verifies stream identity, duration,
coverage, peak/clipping measurements and hashes before outbox publication.
Cancellation and interrupted mux never publish a partial ready file. All codec,
loudness and fit parameters are versioned in the assembly manifest. Final signal
thresholds and consumer versions are frozen before the real pilot.

## Bounded work

The current 512-cue cap is a known implementation restriction, not this endpoint.
The long-story slice must introduce validated configurable source/plan/chunk/byte
limits, a documented hard safety ceiling, a forward SQLite migration and bounded
UI/checkpoint/audio working sets. Page sizes must be bounded and source identity
must survive page changes. Raising a single constant is insufficient acceptance.
Preserve old plans and checkpoint identities during migration; reject unsupported
new data in old binaries. A full-source metadata snapshot may be bounded by the
declared byte limit; decoded audio must be streamed, not loaded whole into RAM.
