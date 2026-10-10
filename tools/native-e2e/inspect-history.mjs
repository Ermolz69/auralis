import fs from 'node:fs';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const root = path.resolve(process.argv[2] ?? '');
if (path.basename(root) !== 'app-data' || !path.basename(path.dirname(root)).startsWith('auralis-native-e2e-'))
  throw new Error('History diagnostics require an isolated native E2E application-data root');
for (const [file, tables] of [
  ['auralis.sqlite', ['project_translations', 'translation_publications', 'artifacts', 'outbox_messages']],
  ['auralis-translate.sqlite', ['results', 'translation_edit_intents']],
]) {
  if (!fs.existsSync(path.join(root, file))) continue;
  const db = new DatabaseSync(path.join(root, file), { readOnly: true });
  try {
    for (const table of tables) {
      if (!db.prepare('SELECT 1 FROM sqlite_master WHERE type = ? AND name = ?').get('table', table)) continue;
      process.stdout.write(`${file} ${table}: ${JSON.stringify(db.prepare(`SELECT * FROM ${table} LIMIT 30`).all())}\n`);
    }
  } finally { db.close(); }
}
