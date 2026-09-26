import { beforeEach, describe, expect, it, vi } from 'vitest';
import { subscribeTranslationChanges } from './translationEvents';

const listen = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ listen }));

type Registration = {
  event: string;
  handler: (event: { payload: unknown }) => void;
  onInvalidPayload: () => void;
  dispose: ReturnType<typeof vi.fn>;
};
let registrations: Registration[];
beforeEach(() => {
  vi.resetAllMocks();
  registrations = [];
  listen.mockImplementation(async (event, handler, options) => {
    const dispose = vi.fn();
    registrations.push({ event, handler, onInvalidPayload: options.onInvalidPayload, dispose });
    return dispose;
  });
});

describe('translation change subscriptions', () => {
  it('filters job kind and project, and includes ready-publication notifications', async () => {
    const refresh = vi.fn();
    const stop = await subscribeTranslationChanges('project', refresh);
    const job = registrations.find((item) => item.event === 'job-event')!;
    for (const [kind, projectId] of [
      ['dubbing', 'project'],
      ['translation', 'other'],
    ]) {
      job.handler({ payload: { job: { kind, projectId } } });
    }
    expect(refresh).not.toHaveBeenCalled();
    job.handler({ payload: { job: { kind: 'translation', projectId: 'project' } } });
    const project = registrations.find((item) => item.event === 'project-updated')!;
    project.handler({ payload: { projectId: 'other' } });
    expect(refresh).toHaveBeenCalledTimes(1);
    project.handler({ payload: { projectId: 'project' } });
    expect(refresh).toHaveBeenCalledTimes(2);
    stop();
    stop();
    for (const registration of registrations) expect(registration.dispose).toHaveBeenCalledTimes(1);
  });

  it('requests authoritative recovery after invalidation or a rejected payload', async () => {
    const refresh = vi.fn();
    const stop = await subscribeTranslationChanges('project', refresh);
    registrations
      .find((item) => item.event === 'job-events-invalidated')!
      .handler({ payload: null });
    for (const registration of registrations) registration.onInvalidPayload();
    expect(refresh).toHaveBeenCalledTimes(4);
    stop();
  });

  it('retains successful listeners when another registration fails', async () => {
    listen.mockRejectedValueOnce(new Error('No job events'));
    const stop = await subscribeTranslationChanges('project', vi.fn());
    expect(registrations).toHaveLength(2);
    stop();
    for (const registration of registrations) expect(registration.dispose).toHaveBeenCalledTimes(1);
  });

  it('aborts registered listeners even while another registration stalls, and closes late arrivals', async () => {
    let resolve: (value: () => void) => void = () => {};
    listen.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const controller = new AbortController();
    const subscription = subscribeTranslationChanges('project', vi.fn(), controller.signal);
    await Promise.resolve();
    await Promise.resolve();
    expect(registrations).toHaveLength(2);
    controller.abort();
    for (const registration of registrations) expect(registration.dispose).toHaveBeenCalledTimes(1);
    const late = vi.fn();
    resolve(late);
    const stop = await subscription;
    expect(late).toHaveBeenCalledTimes(1);
    stop();
    expect(late).toHaveBeenCalledTimes(1);
  });
});
