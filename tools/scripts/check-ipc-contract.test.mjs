import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  assertSameContract,
  assertCommandSignatures,
  extractAnnotatedCommands,
  extractInterfaceKeys,
  extractRegisteredCommands,
  extractRustCommandSignatures,
  extractRustEvents,
  extractTypeScriptCommandSignatures,
  readRustCommandSources,
} from './check-ipc-contract.mjs';

test('finds commands in nested Rust modules', () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-ipc-'));
  try {
    fs.mkdirSync(path.join(directory, 'translation'));
    fs.writeFileSync(
      path.join(directory, 'translation', 'compare.rs'),
      '#[tauri::command]\npub async fn compare_cmd() -> Result<(), Error> {}',
    );
    assert.deepEqual(extractAnnotatedCommands(readRustCommandSources(directory)), ['compare_cmd']);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

test('extracts registered, annotated and typed commands', () => {
  assert.deepEqual(
    extractRegisteredCommands('tauri::generate_handler![commands::one_cmd, commands::two_cmd])'),
    ['one_cmd', 'two_cmd'],
  );
  assert.deepEqual(
    extractAnnotatedCommands([
      '#[command]\npub async fn one_cmd() {}\n#[tauri::command(rename_all = "camelCase")]\npub async fn two_cmd() {}',
    ]),
    ['one_cmd', 'two_cmd'],
  );
  assert.deepEqual(
    extractInterfaceKeys(
      'export interface CommandMap {\n  one_cmd: { args: undefined; result: string };\n  two_cmd: {\n    args: { value: string };\n    result: null;\n  };\n}',
      'CommandMap',
    ),
    ['one_cmd', 'two_cmd'],
  );
});

test('compares command argument names, optionality and result types', () => {
  const rust = extractRustCommandSignatures([
    `#[command]
#[tracing::instrument(skip_all, fields(request_id = %new_id()))]
pub async fn one_cmd(project_id: String, optional: Option<bool>, state: State<'_, App>) -> Result<Vec<ProjectDto>, CommandError> {}
#[tauri::command]
pub async fn empty_cmd(app: tauri::AppHandle) -> Result<(), CommandError> {}`,
  ]);
  const typescript = extractTypeScriptCommandSignatures(`export interface CommandMap {
  one_cmd: {
    args: { projectId: string; optional?: boolean | null };
    result: Project[];
  };
  empty_cmd: { args: undefined; result: null };
}`);
  assert.deepEqual(rust, typescript);
  assert.doesNotThrow(() => assertCommandSignatures(rust, typescript));
  assert.throws(
    () =>
      assertCommandSignatures(rust, {
        ...typescript,
        one_cmd: { args: ['projectId:string'], result: 'Project[]' },
      }),
    /IPC signature drift for one_cmd/,
  );
});

test('extracts the centralized Rust event registry and quoted EventMap keys', () => {
  assert.deepEqual(
    extractRustEvents('pub const EVENT_ONE: &str = "one";\npub const EVENT_TWO: &str = "two";'),
    ['one', 'two'],
  );
  assert.deepEqual(
    extractInterfaceKeys(
      "export interface EventMap {\n  'one': string;\n  'two': undefined;\n}",
      'EventMap',
    ),
    ['one', 'two'],
  );
});

test('checks inherited command maps and Rust string-keyed selections', () => {
  const source = `export interface VoiceCommandMap {
  cast_cmd: { args: { selections: Record<string, Selection>; expectedRevision?: string | null }; result: string };
}
export interface CommandMap extends VoiceCommandMap {
  health_cmd: { args: undefined; result: string };
}`;
  const rust = extractRustCommandSignatures([
    `#[command]
pub async fn cast_cmd(selections: BTreeMap<String, Selection>, expected_revision: Option<String>) -> Result<String, CommandError> {}
#[command]
pub async fn health_cmd() -> Result<String, CommandError> {}`,
  ]);
  assert.deepEqual(extractInterfaceKeys(source, 'CommandMap'), ['cast_cmd', 'health_cmd']);
  assert.doesNotThrow(() =>
    assertCommandSignatures(rust, extractTypeScriptCommandSignatures(source)),
  );
});

test('reports both sides of contract drift', () => {
  assert.throws(
    () => assertSameContract('IPC', ['rust-only', 'shared'], ['shared', 'typescript-only']),
    /Rust-only \[rust-only\], TypeScript-only \[typescript-only\]/,
  );
});

test('distinguishes JSON objects from arbitrary JSON values', () => {
  const rust = extractRustCommandSignatures([
    '#[command] pub async fn object_cmd() -> Result<std::collections::BTreeMap<String, serde_json::Value>, Error> {}',
    '#[command] pub async fn value_cmd() -> Result<serde_json::Value, Error> {}',
  ]);
  assert.equal(rust.object_cmd.result, 'Record<string, unknown>');
  assert.equal(rust.value_cmd.result, 'unknown');
  assert.throws(
    () =>
      assertCommandSignatures(rust, {
        object_cmd: { args: [], result: 'unknown' },
      }),
    /IPC signature drift/,
  );
});

test('keeps inherited command keys and signatures after multiline formatting', () => {
  const source = `export interface VoiceMap {
  voice_cmd: { args: undefined; result: string };
}
export interface MediaMap {
  media_cmd: { args: { id: string }; result: boolean };
}
export interface CommandMap
  extends
    VoiceMap,
    MediaMap {
  own_cmd: { args: undefined; result: null };
}`;
  assert.deepEqual(extractInterfaceKeys(source, 'CommandMap'), ['media_cmd', 'own_cmd', 'voice_cmd']);
  assert.deepEqual(extractTypeScriptCommandSignatures(source).media_cmd, {
    args: ['id:string'], result: 'boolean',
  });
});
