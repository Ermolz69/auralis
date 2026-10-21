import { DatabaseSync } from 'node:sqlite';
import path from 'node:path';
const database = new DatabaseSync(path.join(process.argv[2], 'auralis.sqlite'), { readOnly: true });
try {
  const hasContract = database
    .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='puza_contract_projects'")
    .get();
  if (process.argv.includes('--summary')) {
    process.stdout.write(
      JSON.stringify(
        {
          schema: database.prepare('PRAGMA user_version').get(),
          contract: hasContract
            ? database
                .prepare(
                  "SELECT project_id,json_extract(snapshot_json,'$.contract_version') AS version FROM puza_contract_projects",
                )
                .all()
            : [],
          projects: database.prepare('SELECT title FROM projects').all(),
          runs: database
            .prepare(
              'SELECT profile_id,COUNT(*) AS count FROM speech_run_intents GROUP BY profile_id',
            )
            .all(),
          jobs: database.prepare('SELECT status,COUNT(*) AS count FROM jobs GROUP BY status').all(),
          active: database
            .prepare(
              "SELECT status,progress_json,error_json FROM jobs WHERE status NOT IN ('completed','cancelled') ORDER BY created_at",
            )
            .all(),
          ready: database
            .prepare('SELECT COUNT(*) AS count FROM speech_publications WHERE ready=1')
            .get(),
        },
        null,
        2,
      ),
    );
  } else {
    process.stdout.write(
      JSON.stringify(
        {
          projects: database.prepare('SELECT id,title FROM projects').all(),
          runs: database
            .prepare(
              'SELECT run_id,profile_id,expected_utterances,expected_chunks FROM speech_run_intents ORDER BY created_at',
            )
            .all(),
          jobs: database
            .prepare('SELECT id,status,progress_json,error_json FROM jobs ORDER BY created_at')
            .all(),
          publications: database.prepare('SELECT run_id,ready FROM speech_publications').all(),
        },
        null,
        2,
      ),
    );
  }
} finally {
  database.close();
}
