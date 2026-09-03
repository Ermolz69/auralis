import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const security = JSON.parse(
  readFileSync(new URL('../../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
).app.security;

test('production CSP restricts executable content and explicitly scopes local resources', () => {
  const csp = security.csp;
  assert.equal(csp['default-src'], "'self'");
  assert.equal(csp['script-src'], "'self'");
  for (const directive of ['object-src', 'frame-src', 'base-uri', 'form-action'])
    assert.equal(csp[directive], "'none'");
  for (const directive of ['img-src', 'media-src', 'connect-src'])
    assert.equal(typeof csp[directive], 'string');
  assert.match(csp['connect-src'], /ipc: http:\/\/ipc\.localhost/);
  assert.match(csp['img-src'], /blob: data:/);
  assert.match(csp['media-src'], /asset:/);
  assert.doesNotMatch(JSON.stringify(csp), /unsafe-eval|\*|ws:\/\//);
  assert.equal(security.dangerousDisableAssetCspModification, undefined);
});

test('development policy only adds the fixed Vite HMR endpoint', () => {
  const { 'connect-src': devConnect, ...dev } = security.devCsp;
  const { 'connect-src': prodConnect, ...production } = security.csp;
  assert.deepEqual(dev, production);
  assert.equal(devConnect, `${prodConnect} ws://localhost:5173`);
});
