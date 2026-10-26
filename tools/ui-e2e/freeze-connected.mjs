import fsp from 'node:fs/promises';
import path from 'node:path';
import { storage, idPattern, json, validateSpec } from './core.mjs';
import { candidateIdentity } from './session.mjs';
import { inferenceDraft } from './spec-defaults.mjs';
import { freezeSelections } from './freeze-selections.mjs';

const [caseId, seedId, componentId, mode = 'smoke', scope = 'full-source'] = process.argv.slice(2);
if (
  ![caseId, seedId, componentId].every((value) => idPattern.test(value ?? '')) ||
  !['smoke', 'real'].includes(mode)
)
  throw new Error('Pin case, seed, components and supported mode');
const candidate = await candidateIdentity();
const runtime_seed = await json(path.join(storage, 'seeds', `${seedId}.json`), 32 * 1024 * 1024);
const components = await json(path.join(storage, 'wheels', componentId, 'components.json'));
const spec = {
  version: 1,
  case_id: caseId,
  candidate_sha256: candidate.receipt.sha256,
  candidate_receipt_sha256: candidate.receipt_sha256,
  max_attempts: 1,
  runtime_seed,
  component_wheels: components.files,
  component_commits: { contract: components.contract_commit, tts: components.tts_commit },
  allow_inference: false,
  max_wall_seconds: mode === 'real' ? 28800 : 1800,
};
if (mode === 'smoke') spec.mode = 'component-smoke';
else {
  Object.assign(spec, {
    ...inferenceDraft(scope),
    source_url: 'https://www.youtube.com/watch?v=HRaa8kb1EhE',
    source_subtitle_sha256: '868dc24073183cfd9e5f41ded3eb0cca34b6faad53e008a8b50ddb6f79c49735',
    track: { language: 'zh', format: 'srt', automatic: false },
    edit_text: 'В центре Сеула, на перекрёстке улицы Ханбок.',
    max_source_media_bytes: 1073741824,
    allow_unverified_neutral: true,
    engines: [
      {
        engine_id: 'voxcpm2',
        profile_id: 'native-plan-voxcpm2',
        voice_id: 'marina_elevenlabs_ru',
        reference_id: 'ref-voxcpm2-neutral',
      },
      {
        engine_id: 'espeech',
        profile_id: 'native-plan-espeech',
        voice_id: 'alex_analyst_elevenlabs_ru',
        reference_id: 'ref-espeech-neutral',
      },
    ],
  });
  if (spec.install_translation)
    spec.runtime_seed = {
      ...runtime_seed,
      files: runtime_seed.files.filter(
        (item) =>
          !['translation-runtime.json', 'translation-package-selection.json'].includes(
            item.path.toLowerCase(),
          ),
      ),
    };
  spec.engine_identities = await freezeSelections(runtime_seed, components.files, spec.engines);
  validateSpec(spec);
}
const file = path.join(storage, 'specs', `${caseId}.json`);
await fsp.mkdir(path.dirname(file), { recursive: true });
await fsp.writeFile(file, JSON.stringify(spec, null, 2), { flag: 'wx' });
process.stdout.write(`Frozen connected ${mode} case: ${path.relative(process.cwd(), file)}\n`);
