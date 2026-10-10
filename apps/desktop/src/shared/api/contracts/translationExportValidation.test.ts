import { expect, it } from 'vitest';
import { parseCommandResult } from './runtimeValidation';

const receipt = {
  resultId: '00000000-0000-4000-8000-000000000001',
  resultSha256: 'a'.repeat(64),
  sizeBytes: 120,
  destination: 'C:\\Exports\\translation.srt',
};

it('accepts an exact export receipt and rejects changed result identity', () => {
  expect(parseCommandResult('export_translation_result_cmd', receipt)).toEqual(receipt);
  expect(() =>
    parseCommandResult('export_translation_result_cmd', {
      ...receipt,
      resultSha256: 'not-a-digest',
    }),
  ).toThrow();
  expect(() =>
    parseCommandResult('export_translation_result_cmd', {
      ...receipt,
      sizeBytes: Number.MAX_SAFE_INTEGER + 1,
    }),
  ).toThrow();
});
