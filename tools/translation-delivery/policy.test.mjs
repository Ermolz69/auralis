import assert from 'node:assert/strict';
import test from 'node:test';
import { assertNoTranslationPayload, assertProductionFrontend, assertProductionResources } from './policy.mjs';

test('weight files and local evaluation data cannot enter the native bundle', () => {
  for (const file of ['model/Hy-MT2.gguf', 'model.safetensors', '.cache/eval/report.json', 'eval/source.srt', 'tests/fixtures/input.srt']) {
    assert.throws(() => assertNoTranslationPayload([file]), /must not be bundled/u);
  }
  assertNoTranslationPayload(['binaries/ffmpeg.exe', 'profile.json', 'notices/LLAMA-LICENSE.txt']);
});

test('dead test imports are allowed but emitted test code fails', () => {
  const module = { id: 'src/app/native-e2e/NativeE2ERunner.tsx', renderedBytes: 0 };
  assertProductionFrontend([{ modules: [module] }], ['assets/index.js']);
  assert.throws(() => assertProductionFrontend([{ modules: [{ ...module, renderedBytes: 40 }] }], ['assets/index.js']), /Test code was emitted/u);
});

test('unbounded resources cannot pull repository data into an installer', () => {
  assertProductionResources({ bundle: { resources: ['binaries/ffmpeg-*'] } });
  assert.throws(() => assertProductionResources({ bundle: { resources: ['**/*'] } }), /bounded media-tool/u);
  assert.throws(() => assertProductionResources({ bundle: { resources: ['models/*.gguf'] } }), /must not be bundled/u);
});
