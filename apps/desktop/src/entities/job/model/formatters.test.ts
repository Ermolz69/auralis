import { describe, expect, it } from 'vitest';
import { formatJobStage, formatJobStatus, getJobStatusTone, isActiveJobStatus } from './formatters';

describe('job presentation formatters', () => {
  it('maps job statuses and known stages to user-facing labels', () => {
    expect(formatJobStatus({ status: 'pending', stage: 'downloadMedia' })).toBe(
      'Waiting to start: Downloading media',
    );
    expect(formatJobStatus({ status: 'running', stage: 'importYoutubeSubtitles' })).toBe(
      'Running: Importing YouTube subtitles',
    );
    expect(formatJobStatus({ status: 'completed', stage: 'exportResult' })).toBe('Completed');
  });

  it('humanizes unknown technical stages without rendering raw snake case', () => {
    expect(formatJobStage('prepare_audio_mix')).toBe('Prepare audio mix');
    expect(formatJobStage('prepareAudioMix')).toBe('Prepare audio mix');
  });

  it('maps job statuses to semantic tones and active state', () => {
    expect(getJobStatusTone('completed')).toBe('success');
    expect(getJobStatusTone('failed')).toBe('danger');
    expect(getJobStatusTone('cancelled')).toBe('warning');
    expect(getJobStatusTone('running')).toBe('default');
    expect(isActiveJobStatus('pending')).toBe(true);
    expect(isActiveJobStatus('completed')).toBe(false);
  });
});
