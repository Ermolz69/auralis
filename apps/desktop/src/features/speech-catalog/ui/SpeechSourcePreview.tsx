import type { SpeechAssignment, SpeechSourceSnapshot } from '@/shared/api/contracts/speech';
import type { SpeechVoiceOption } from '@/shared/api/contracts/speechCatalog';
type Props = {
  source: SpeechSourceSnapshot | null;
  voices: SpeechVoiceOption[];
  assignments: SpeechAssignment[];
  assign: (scope: 'role' | 'cue', target: string, voice: string) => void;
};
export function SpeechSourcePreview({ source, voices, assignments, assign }: Props) {
  const assignmentSelect = (scope: 'role' | 'cue', target: string) => (
    <select
      aria-label={`Голос ${scope === 'role' ? 'роли' : 'реплики'} ${target}`}
      className="rounded border border-border bg-surface p-1 text-text"
      value={assignments.find((a) => a.scope === scope && a.target === target)?.voice_id ?? ''}
      onChange={(e) => assign(scope, target, e.target.value)}
    >
      <option value="">Общий голос</option>
      {voices.map((v) => (
        <option key={v.voice_id} value={v.voice_id}>
          {v.display_name}
        </option>
      ))}
    </select>
  );
  return (
    <>
      {source && (
        <div className="max-h-64 space-y-2 overflow-auto rounded border border-border p-2 text-text">
          {source.cues.length > 0 ? (
            <>
              {[...new Set(source.cues.map((c) => c.role))].map((role) => (
                <label key={role} className="flex justify-between gap-2">
                  Роль: {role}
                  {assignmentSelect('role', role)}
                </label>
              ))}
              {source.cues.map((cue, q) => (
                <div key={cue.id} className="border-t border-border pt-2">
                  <div className="flex justify-between gap-2">
                    <span>
                      {q + 1} · {(cue.start_ms / 1000).toFixed(2)}–{(cue.end_ms / 1000).toFixed(2)}{' '}
                      с
                    </span>
                    {assignmentSelect('cue', cue.id)}
                  </div>
                  <p className="mt-1 whitespace-pre-wrap">{cue.text}</p>
                </div>
              ))}
            </>
          ) : (
            <p className="whitespace-pre-wrap">{source.text}</p>
          )}
        </div>
      )}
    </>
  );
}
