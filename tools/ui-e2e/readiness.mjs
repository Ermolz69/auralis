import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

export async function profileFacts(profile) {
  const configuration = {};
  for (const name of [
    'tts-runtime.json',
    'translation-runtime.json',
    'translation-package-selection.json',
    'project-runtime.json',
  ]) {
    configuration[name] = await fsp
      .lstat(path.join(profile, name))
      .then((stat) => stat.isFile())
      .catch((error) => {
        if (error.code === 'ENOENT') return false;
        throw error;
      });
  }
  const db = new DatabaseSync(path.join(profile, 'auralis.sqlite'), { readOnly: true });
  try {
    return {
      configuration_files: configuration,
      schema: db.prepare('PRAGMA user_version').get().user_version,
      projects: db.prepare('SELECT COUNT(*) AS n FROM projects').get().n,
      jobs: db.prepare('SELECT COUNT(*) AS n FROM jobs').get().n,
    };
  } finally {
    db.close();
  }
}
