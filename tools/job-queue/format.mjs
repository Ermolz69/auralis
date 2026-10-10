import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const directories = [
  'crates/jobs/src/queue',
  'crates/adapters-storage/src/sqlite/job_executions',
  'crates/application/tests/job_queue',
];
const files = [
  'crates/ports/src/job_execution.rs',
  'crates/ports/src/job_resources.rs',
  'crates/ports/src/job_runtime_control.rs',
  'crates/ports/src/error.rs',
  'crates/ports/src/source.rs',
  'crates/ports/src/lib.rs',
  'crates/jobs/src/lib.rs',
  'crates/jobs/src/manager/manager.rs',
  'crates/jobs/src/manager/runtime_registry.rs',
  'crates/jobs/src/manager/runtime_control.rs',
  'crates/jobs/src/manager/outcome.rs',
  'crates/jobs/src/manager/cleanup.rs',
  'crates/jobs/src/manager/scheduler_impl.rs',
  'crates/domain/src/job/entity.rs',
  'crates/domain/src/job/entity/cleanup.rs',
  'crates/domain/src/media/subtitle.rs',
  'crates/application/src/error.rs',
  'crates/application/src/usecases/system/recover_interrupted/usecase.rs',
  'crates/application/src/usecases/system/recover_interrupted/apply_error.rs',
  'crates/application/src/usecases/pipeline/start_mock.rs',
  'crates/application/src/usecases/pipeline/mock_dubbing_pipeline.rs',
  'crates/application/src/usecases/pipeline/queued_subtitle_executor.rs',
  'crates/application/src/usecases/pipeline/mod.rs',
  'crates/application/src/usecases/transcript/import_youtube_subtitles/usecase.rs',
  'crates/application/src/usecases/transcript/import_youtube_subtitles/revalidation_tests.rs',
  'crates/application/tests/job_queue.rs',
  'crates/application/tests/job_queue_recovery.rs',
  'crates/application/tests/job_queue_benchmark.rs',
  'crates/application/tests/support/job_queue.rs',
  'crates/application/tests/support/queue_subtitles.rs',
  'crates/adapters-storage/src/sqlite/connection.rs',
  'crates/adapters-storage/src/sqlite/mod.rs',
  'crates/adapters-storage/src/sqlite/unit_of_work/mod.rs',
  'crates/adapters-storage/src/sqlite/unit_of_work/job_writes.rs',
  'crates/adapters-storage/src/sqlite/unit_of_work/project_writes.rs',
  'crates/adapters-ytdlp/src/ytdlp/adapter.rs',
  'crates/adapters-ytdlp/src/ytdlp/command.rs',
  'crates/adapters-ytdlp/src/ytdlp/mod.rs',
  'crates/adapters-ytdlp/src/ytdlp/subtitle_process.rs',
  'crates/adapters-ytdlp/src/ytdlp/owned_process/mod.rs',
  'crates/adapters-ytdlp/src/ytdlp/owned_process/cancellable.rs',
  'crates/adapters-ytdlp/src/ytdlp/owned_process/queue_tests.rs',
  'crates/adapters-ytdlp/src/ytdlp/owned_process/windows.rs',
  'crates/adapters-ytdlp/src/ytdlp/owned_process/unix.rs',
  'src-tauri/src/bootstrap/mod.rs',
  'src-tauri/src/bootstrap/services.rs',
  'src-tauri/src/bootstrap/storage.rs',
  'src-tauri/src/bootstrap/usecases.rs',
  'src-tauri/src/bootstrap/native_e2e_subtitle_source.rs',
  'src-tauri/src/dto/error.rs',
];
for (const directory of directories) {
  files.push(...readdirSync(directory).filter((name) => name.endsWith('.rs')).map((name) => join(directory, name)));
}
const result = spawnSync('rustfmt', ['--edition', '2024', '--config', 'skip_children=true', ...files], { stdio: 'inherit' });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
