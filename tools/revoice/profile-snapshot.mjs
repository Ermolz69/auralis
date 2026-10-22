// Engineering-only backup of the ordinary local profile. No model or app input.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { DatabaseSync, backup } from 'node:sqlite';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const profile = path.join(process.env.APPDATA, 'com.auralis.desktop');
const hash = async (file) => {
  const digest = crypto.createHash('sha256');
  for await (const chunk of fs.createReadStream(file)) digest.update(chunk);
  return digest.digest('hex');
};
const database = path.join(profile, 'auralis.sqlite');
const mode = process.argv[2];
if (!['inspect', 'backup'].includes(mode)) throw new Error('Use inspect or backup OUTPUT');
const db = new DatabaseSync(database, { readOnly: true });
try {
  const counts = {
    projects: db.prepare('SELECT COUNT(*) AS n FROM projects').get().n,
    jobs: db.prepare('SELECT COUNT(*) AS n FROM jobs').get().n,
    active_jobs: db
      .prepare(
        "SELECT COUNT(*) AS n FROM jobs WHERE lower(status) NOT IN ('completed','cancelled','failed')",
      )
      .get().n,
    schema: db.prepare('PRAGMA user_version').get().user_version,
  };
  const record = { version: 1, at: new Date().toISOString(), profile, ...counts, backups: [] };
  if (mode === 'backup') {
    if (counts.active_jobs) throw new Error('Refuse upgrade backup while jobs are active');
    const output = path.resolve(root, process.argv[3] ?? '');
    const allowed = path.join(root, 'target/revoice') + path.sep;
    if (!output.startsWith(allowed) || fs.existsSync(output))
      throw new Error('Backup must be a new directory under target/revoice');
    fs.mkdirSync(output);
    const destination = path.join(output, 'auralis.sqlite');
    await backup(db, destination);
    const check = new DatabaseSync(destination, { readOnly: true });
    try {
      if (check.prepare('PRAGMA quick_check').get().quick_check !== 'ok')
        throw new Error('Database backup failed quick_check');
      if (check.prepare('SELECT COUNT(*) AS n FROM projects').get().n !== counts.projects)
        throw new Error('Profile changed during backup; stop before upgrade');
    } finally {
      check.close();
    }
    record.backups.push({
      source: database,
      copy: destination,
      sha256: await hash(destination),
      kind: 'SQLite online backup, WAL included',
    });
    const translation = path.join(profile, 'auralis-translate.sqlite');
    if (fs.existsSync(translation)) {
      const source = new DatabaseSync(translation, { readOnly: true });
      const copy = path.join(output, 'auralis-translate.sqlite');
      try {
        await backup(source, copy);
      } finally {
        source.close();
      }
      record.backups.push({
        source: translation,
        copy,
        sha256: await hash(copy),
        kind: 'SQLite online backup, WAL included',
      });
    }
    for (const name of [
      'tts-runtime.json',
      'translation-runtime.json',
      'translation-package-selection.json',
      'caster-runtime.json',
      'puza-marker-runtime.json',
      'speech-limits.json',
    ]) {
      const source = path.join(profile, name);
      if (!fs.existsSync(source)) continue;
      if (!fs.lstatSync(source).isFile() || fs.statSync(source).size > 1_048_576)
        throw new Error(`Unbounded configuration: ${name}`);
      const copy = path.join(output, name);
      fs.copyFileSync(source, copy, fs.constants.COPYFILE_EXCL);
      if ((await hash(source)) !== (await hash(copy)))
        throw new Error(`Configuration changed during backup: ${name}`);
      record.backups.push({ source, copy, sha256: await hash(copy), kind: 'configuration copy' });
    }
    fs.writeFileSync(path.join(output, 'receipt.json'), JSON.stringify(record, null, 2) + '\n', {
      flag: 'wx',
    });
  }
  process.stdout.write(JSON.stringify(record) + '\n');
} finally {
  db.close();
}
