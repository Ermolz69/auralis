import assert from 'node:assert/strict';
import test from 'node:test';
import { readScenarioConfig, successMessage } from './scenario-config.mjs';

const prefix = 'AURALIS_NATIVE_E2E_';
const envFor = (...flags) => Object.fromEntries(flags.map((flag) => [`${prefix}${flag}`, '1']));
const enabled = (config) =>
  Object.entries(config)
    .filter(([, value]) => value)
    .map(([name]) => name);

const translation = 'TRANSLATION';
const history = 'TRANSLATION_HISTORY';
const preparation = 'TRANSLATION_PREPARATION_PAUSE';
const initialHash = 'TRANSLATION_INITIAL_HASH_PAUSE';
const crash = 'TRANSLATION_CRASH';

const scenarios = [
  ['media', [], []],
  ['caster', ['CASTER'], ['casterEnabled']],
  ['translation', [translation], ['translationEnabled']],
  [
    'events',
    [translation, 'TRANSLATION_EVENTS'],
    ['translationEnabled', 'translationEventsEnabled'],
  ],
  ['history', [translation, history], ['translationEnabled', 'translationHistoryEnabled']],
  [
    'history export',
    [translation, history, 'TRANSLATION_HISTORY_EXPORT'],
    ['translationEnabled', 'translationHistoryEnabled', 'historicalExportEnabled'],
  ],
  [
    'WebVTT history export',
    [translation, 'TRANSLATION_VTT', history, 'TRANSLATION_HISTORY_EXPORT'],
    [
      'translationEnabled',
      'translationHistoryEnabled',
      'historicalExportEnabled',
      'translationVttEnabled',
    ],
  ],
  [
    'history result gap',
    [translation, history, 'TRANSLATION_HISTORY_GAP'],
    ['translationEnabled', 'translationHistoryEnabled', 'historicalGapEnabled'],
  ],
  [
    'history journal gap',
    [translation, history, 'TRANSLATION_HISTORY_GAP', 'TRANSLATION_HISTORY_JOURNAL_GAP'],
    [
      'translationEnabled',
      'translationHistoryEnabled',
      'historicalGapEnabled',
      'historicalJournalOnly',
    ],
  ],
  [
    'package',
    [translation, 'TRANSLATION_PACKAGE'],
    ['translationEnabled', 'translationPackageEnabled'],
  ],
  ['pause', [translation, 'TRANSLATION_PAUSE'], ['translationEnabled', 'translationPauseEnabled']],
  ['WebVTT', [translation, 'TRANSLATION_VTT'], ['translationEnabled', 'translationVttEnabled']],
  [
    'preparation pause',
    [translation, preparation],
    ['translationEnabled', 'translationPreparationPauseEnabled'],
  ],
  [
    'worker preflight',
    [translation, preparation, 'TRANSLATION_WORKER_PREFLIGHT_PAUSE'],
    [
      'translationEnabled',
      'translationPreparationPauseEnabled',
      'translationWorkerPreflightPauseEnabled',
    ],
  ],
  [
    'initial hash',
    [translation, preparation, initialHash],
    [
      'translationEnabled',
      'translationPreparationPauseEnabled',
      'translationInitialHashPauseEnabled',
    ],
  ],
  [
    'start race',
    [translation, preparation, initialHash, 'TRANSLATION_START_RACE'],
    [
      'translationEnabled',
      'translationPreparationPauseEnabled',
      'translationInitialHashPauseEnabled',
      'translationStartRaceEnabled',
    ],
  ],
  [
    'WebVTT pause',
    [translation, 'TRANSLATION_VTT', 'TRANSLATION_PAUSE'],
    ['translationEnabled', 'translationVttEnabled', 'translationPauseEnabled'],
  ],
  [
    'WebVTT crash',
    [translation, 'TRANSLATION_VTT', crash],
    ['translationEnabled', 'translationVttEnabled', 'translationCrashEnabled'],
  ],
  ['crash', [translation, crash], ['translationEnabled', 'translationCrashEnabled']],
  [
    'result gap',
    [translation, crash, 'TRANSLATION_RESULT_GAP'],
    ['translationEnabled', 'translationCrashEnabled', 'translationResultGapEnabled'],
  ],
  [
    'delete',
    [translation, 'TRANSLATION_DELETE'],
    ['translationEnabled', 'translationDeleteEnabled'],
  ],
  ['single owner', ['SECOND_OWNER'], ['secondOwnerEnabled']],
];

const successPrefixes = [
  'Native Tauri E2E passed:',
  'Native Caster E2E passed:',
  'Native Tauri E2E passed:',
  'Native Tauri E2E passed:',
  'Native Tauri history E2E passed:',
  'Native Tauri history export E2E passed:',
  'Native Tauri history export E2E passed:',
  'Native Tauri history E2E passed:',
  'Native Tauri history E2E passed:',
  'Native Tauri package E2E passed:',
  'Native Tauri pause E2E passed:',
  'Native Tauri WebVTT E2E passed:',
  'Native preparation pause E2E passed:',
  'Native worker preflight pause E2E passed:',
  'Native initial hash pause E2E passed:',
  'Native concurrent starts E2E passed:',
  'Native Tauri pause E2E passed:',
  'Native Tauri WebVTT crash E2E passed:',
  'Native Tauri crash E2E passed:',
  'Native Tauri result-gap E2E passed:',
  'Native Tauri deletion E2E passed:',
  'Native Tauri single-owner E2E passed:',
];

for (const [index, [name, flags, expected]] of scenarios.entries()) {
  test(`${name} retains its scenario flags`, () => {
    const env = envFor(...flags);
    assert.deepEqual(enabled(readScenarioConfig(env)), expected);
    assert.ok(successMessage(env).startsWith(successPrefixes[index]));
    assert.ok(successMessage(env).endsWith('; sandbox cleaned.\n'));
  });
}

const invalid = [
  [['CASTER', translation], 'Caster native E2E runs without Translate'],
  [
    ['TRANSLATION_HISTORY_JOURNAL_GAP'],
    'Journal-only interruption requires the historical gap mode',
  ],
  [['TRANSLATION_HISTORY_EXPORT'], 'Historical export requires the ordinary SRT history mode'],
  [
    [translation, history, 'TRANSLATION_HISTORY_GAP', 'TRANSLATION_HISTORY_EXPORT'],
    'Historical export requires the ordinary SRT history mode',
  ],
  [['TRANSLATION_HISTORY_GAP'], 'Historical core gap requires the standalone history mode'],
  [[history], 'Translation history requires a separate ordinary SRT invocation'],
  [
    [translation, history, 'TRANSLATION_EVENTS'],
    'Translation history requires a separate ordinary SRT invocation',
  ],
  [['TRANSLATION_EVENTS'], 'Translation events require a separate ordinary SRT invocation'],
  [
    [translation, 'TRANSLATION_EVENTS', 'TRANSLATION_VTT'],
    'Translation events require a separate ordinary SRT invocation',
  ],
  [
    ['TRANSLATION_START_RACE'],
    'Concurrent start observation requires the initial-hash preparation mode',
  ],
  [[initialHash], 'Initial hash pause requires a separate preparation-pause invocation'],
  [
    [translation, preparation, initialHash, 'TRANSLATION_WORKER_PREFLIGHT_PAUSE'],
    'Initial hash pause requires a separate preparation-pause invocation',
  ],
  [
    ['TRANSLATION_WORKER_PREFLIGHT_PAUSE'],
    'Worker preflight pause requires the preparation-pause mode',
  ],
  [
    [translation, preparation, 'TRANSLATION_PAUSE'],
    'Native preparation pause runs separately from inference pause, crash, deletion and package modes',
  ],
  [
    [translation, 'TRANSLATION_PAUSE', crash],
    'Native pause acceptance does not combine with crash, deletion or package modes',
  ],
  [
    [translation, 'TRANSLATION_VTT', crash, 'TRANSLATION_RESULT_GAP'],
    'Native WebVTT acceptance does not combine with result-gap or deletion modes',
  ],
  [
    [translation, 'TRANSLATION_VTT', 'TRANSLATION_PACKAGE'],
    'Native package acceptance runs as a standalone translation mode',
  ],
];

for (const [flags, message] of invalid) {
  test(`rejects ${flags.join(' + ')} with the existing error`, () => {
    assert.throws(() => readScenarioConfig(envFor(...flags)), { message });
  });
}

test('non-1 values and dependent flags retain their original gating', () => {
  assert.deepEqual(
    enabled(readScenarioConfig(envFor('TRANSLATION_VTT', 'TRANSLATION_RESULT_GAP'))),
    [],
  );
  assert.deepEqual(enabled(readScenarioConfig({ [`${prefix}${translation}`]: 'true' })), []);
});
