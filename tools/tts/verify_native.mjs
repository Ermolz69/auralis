import { verifySpeechArtifacts } from '../native-e2e/speech.mjs';

const expected = Number(process.argv[3]);
if (!Number.isSafeInteger(expected) || expected < 1)
  throw new Error('Expected run count must be a positive integer');
await verifySpeechArtifacts(process.argv[2], expected);
process.stdout.write(
  `Verified ${expected} ready native speech results and their retained hashes.\n`,
);
