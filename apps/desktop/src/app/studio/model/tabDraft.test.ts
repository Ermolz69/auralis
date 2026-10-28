import { expect, it } from 'vitest';
import { parseDraft } from './tabDraft';
it.each(['broken', 'null', '[]', '42', '"text"', '{"text":42}'])(
  'ignores an invalid saved draft %s',
  (raw) => {
    expect(parseDraft(raw)).toEqual({});
  },
);
it('preserves an intentionally empty edit and saved source selections', () => {
  const draft = {
    text: '',
    mode: 'subtitles',
    subtitleId: 'artifact',
    translationId: 'translation',
    selected: 'run',
  };
  expect(parseDraft(JSON.stringify(draft))).toEqual(draft);
});
