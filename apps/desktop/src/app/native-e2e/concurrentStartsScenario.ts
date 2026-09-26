import { getTranslationRunStatus } from '@/entities/translation';
import { isCommandError } from '@/shared/api/contracts/error';
import { invoke } from '@/shared/api/tauri';

const CONCURRENT_START_COUNT = 2;

export async function concurrentStartsScenario(
  projectId: string,
  translationId: string,
  runId: string,
  activeJobId: string | null,
): Promise<void> {
  const replies = await Promise.allSettled(
    Array.from({ length: CONCURRENT_START_COUNT }, () =>
      invoke('start_translation_run_cmd', { projectId, translationId }),
    ),
  );
  const outcomes = replies.map((reply) => {
    if (activeJobId === null) {
      if (
        reply.status !== 'rejected' ||
        !isCommandError(reply.reason) ||
        reply.reason.code !== 'BUSY'
      ) {
        throw new Error('A concurrent start bypassed the owned model preparation slot');
      }
      return { status: reply.status, code: reply.reason.code };
    }
    if (
      reply.status !== 'fulfilled' ||
      reply.value.state !== 'accepted' ||
      reply.value.jobId !== activeJobId ||
      reply.value.runId !== runId
    ) {
      throw new Error('A concurrent start did not reuse the accepted host job');
    }
    return { status: reply.status, ...reply.value };
  });
  const status = await getTranslationRunStatus(projectId, translationId);
  if (
    status.runId !== runId ||
    status.activeHostJobId !== activeJobId ||
    status.phase !== (activeJobId === null ? 'requested' : 'paused') ||
    status.pauseRequested ||
    status.savedBlocks !== 0 ||
    status.resultId
  ) {
    throw new Error('Concurrent starts changed the retained preparation state');
  }
  await invoke('native_e2e_checkpoint_cmd', {
    checkpoint: `translation-concurrent-starts:${JSON.stringify({
      boundary: activeJobId === null ? 'initial_hash' : 'worker_preflight',
      runId,
      outcomes,
    })}`,
  });
}
