import fsp from 'node:fs/promises';
import path from 'node:path';
import { storage, idPattern, validateSpec } from './core.mjs';
import { candidateIdentity } from './session.mjs';

const caseId = process.argv[2];
if (!idPattern.test(caseId ?? '')) throw new Error('CASE must be a new stable case ID');
const candidate = await candidateIdentity();
const spec = {
  version: 1,
  case_id: caseId,
  candidate_sha256: candidate.receipt.sha256,
  source_url: 'https://www.youtube.com/watch?v=HRaa8kb1EhE',
  source_subtitle_sha256: '868dc24073183cfd9e5f41ded3eb0cca34b6faad53e008a8b50ddb6f79c49735',
  track: { language: 'zh', format: 'srt', automatic: false },
  edit_text: 'Автоматическая проверка сохранения отдельной версии.',
  max_wall_seconds: 1200,
  allow_inference: false,
  engines: ['voxcpm2', 'espeech'].map((engine) => ({
    engine_id: engine,
    profile_id: 'unconfigured',
    voice_id: 'unconfigured',
    reference_id: 'unconfigured',
  })),
};
validateSpec(spec);
const file = path.join(storage, 'specs', `${caseId}.json`);
await fsp.mkdir(path.dirname(file), { recursive: true });
await fsp.writeFile(file, JSON.stringify(spec, null, 2), { flag: 'wx' });
process.stdout.write(`Frozen readiness-only case: ${path.relative(process.cwd(), file)}\n`);
