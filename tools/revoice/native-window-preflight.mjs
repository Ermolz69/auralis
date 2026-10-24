import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { DatabaseSync } from 'node:sqlite';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const relative = process.argv[2] ?? 'tools/revoice/native-window-20261024-v1.json';
if (!/^tools\/revoice\/native-window-20261024-v[12]\.json$/.test(relative))
  throw new Error('Unknown frozen window audit');
const spec = JSON.parse(fs.readFileSync(path.join(root, relative), 'utf8'));
const committed = JSON.parse(
  execFileSync('git', ['show', `HEAD:${relative}`], {
    cwd: root,
    encoding: 'utf8',
    timeout: 10000,
  }),
);
if (JSON.stringify(spec) !== JSON.stringify(committed))
  throw new Error('Freeze the exact window audit before observation');
if (
  spec.max_capture_attempts !== 2 ||
  spec.max_capture_seconds_each !== 30 ||
  spec.max_total_seconds !== 120 ||
  spec.max_existing_app_launches !== 1
)
  throw new Error('Unexpected finite window budget');
const application = path.join(process.env.LOCALAPPDATA, 'Programs', 'Auralis', 'auralis-app.exe');
const metadata = fs.lstatSync(application);
if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size > 64 * 1024 * 1024)
  throw new Error('Installed application is not a bounded regular file');
const digest = crypto.createHash('sha256');
for await (const chunk of fs.createReadStream(application)) digest.update(chunk);
const sha256 = digest.digest('hex');
if (sha256 !== spec.expected_application_sha256)
  throw new Error('Installed binary changed; stop this audit');
const db = new DatabaseSync(
  path.join(process.env.APPDATA, 'com.auralis.desktop', 'auralis.sqlite'),
  { readOnly: true },
);
let profile;
try {
  profile = {
    schema: db.prepare('PRAGMA user_version').get().user_version,
    projects: db.prepare('SELECT COUNT(*) AS n FROM projects').get().n,
    active_jobs: db
      .prepare(
        "SELECT COUNT(*) AS n FROM jobs WHERE lower(status) NOT IN ('completed','cancelled','failed')",
      )
      .get().n,
  };
} finally {
  db.close();
}
if (JSON.stringify(profile) !== JSON.stringify(spec.profile_precondition))
  throw new Error('Profile precondition changed; do not launch or capture');
process.stdout.write(
  JSON.stringify({
    case_id: spec.case_id,
    at: new Date().toISOString(),
    installed_application_sha256: sha256,
    profile,
    capture_budget: spec.max_capture_attempts,
    read_only_preflight: true,
  }) + '\n',
);
