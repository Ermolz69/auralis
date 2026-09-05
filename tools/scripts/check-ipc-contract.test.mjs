import assert from 'node:assert/strict';
import test from 'node:test';
import {
  assertSameContract,
  assertCommandSignatures,
  extractAnnotatedCommands,
  extractInterfaceKeys,
  extractRegisteredCommands,
  extractRustCommandSignatures,
  extractRustEvents,
  extractTypeScriptCommandSignatures,
} from './check-ipc-contract.mjs';

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
    extractRustEvents(
      'pub const EVENT_ONE: &str = "one";\npub const EVENT_TWO: &str = "two";',
    ),
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

test('reports both sides of contract drift', () => {
  assert.throws(
    () => assertSameContract('IPC', ['rust-only', 'shared'], ['shared', 'typescript-only']),
    /Rust-only \[rust-only\], TypeScript-only \[typescript-only\]/,
  );
});
