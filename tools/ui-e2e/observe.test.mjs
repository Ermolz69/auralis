import assert from 'node:assert/strict';
import { test } from 'node:test';
import { clickOnce, jobState, readyOrError } from './observe.mjs';
import { Blocked } from './core.mjs';

test('an accepted click timeout is observed without a second dispatch', async () => {
  let clicks = 0;
  const control = {
    click: async () => {
      clicks++;
      throw Object.assign(new Error('Timed out after dispatch'), { name: 'TimeoutError' });
    },
  };
  assert.equal(
    (await clickOnce(control, async () => ({ job: 'accepted' }))).click,
    'ambiguous-confirmed',
  );
  assert.equal(clicks, 1);
  await assert.rejects(
    clickOnce(control, async () => {
      throw new Error('No job');
    }),
    /No job.*ambiguous/,
  );
  assert.equal(clicks, 2);
});

const scope = (state, alert = null) => ({
  getByRole: (role) => ({
    first() {
      return this;
    },
    isVisible: async () => role === 'status' || alert !== null,
    innerText: async () => (role === 'status' ? `Задание fixture · ${state}` : alert),
  }),
});

test('a failed job with a resume button cannot count as cancellation; early completion is blocked', async () => {
  await assert.rejects(jobState(scope('ошибка'), 'Состояние озвучки', ['отменено'], 100), /ошибка/);
  await assert.rejects(jobState(scope('готово'), 'Состояние озвучки', ['отменено'], 100), Blocked);
  assert.equal(
    (await jobState(scope('отменено'), 'Состояние озвучки', ['отменено'], 100)).state,
    'отменено',
  );
});

test('a visible error wins over an old ready control', async () => {
  await assert.rejects(
    readyOrError({ isVisible: async () => true }, scope('ошибка', 'Damaged artifact'), 100),
    /Damaged artifact/,
  );
});

test('async resume ignores the previous cancelled job until a different job is observed', async () => {
  let reads = 0;
  const view = {
    getByRole: (role) => ({
      first() {
        return this;
      },
      isVisible: async () => role === 'status',
      innerText: async () => (++reads < 3 ? 'Задание old · отменено' : 'Задание new · выполняется'),
    }),
  };
  const result = await jobState(view, 'Состояние озвучки', ['выполняется', 'готово'], 1000, {
    job_id: 'old',
  });
  assert.equal(result.job_id, 'new');
  assert.equal(reads, 3);
});
