import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { test } from 'node:test';
import { owned, root, storage } from './core.mjs';

test('offline component builds bind exact clean sources and wheel bytes', async (t) => {
  assert.equal(process.platform, 'win32', 'This task verifies the Windows PowerShell builder');
  const python = process.env.AURALIS_COMPONENT_BUILD_PYTHON;
  const uv = process.env.AURALIS_COMPONENT_BUILD_UV;
  assert.ok(python && uv, 'Select explicit model-free Python and uv build tooling');
  const base = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-component-build-'));
  const outputs = new Set();
  const env = {
    ...process.env,
    SOURCE_DATE_EPOCH: '1791639837',
    UV_PYTHON_DOWNLOADS: 'never',
    UV_CACHE_DIR: path.join(base, 'uv-cache'),
    UV_LINK_MODE: 'copy',
    PYTHONDONTWRITEBYTECODE: '1',
  };
  const git = (cwd, ...args) =>
    execFileSync('git', ['-C', cwd, ...args], { encoding: 'utf8', windowsHide: true }).trim();
  const hash = async (file) =>
    crypto
      .createHash('sha256')
      .update(await fsp.readFile(file))
      .digest('hex');
  const fixture = async (name, folder, distribution, version, dependencies = '') => {
    const repository = path.join(base, name);
    const project = path.join(repository, folder);
    const module = distribution.replaceAll('-', '_');
    await fsp.mkdir(path.join(project, 'src', module), { recursive: true });
    await fsp.writeFile(path.join(repository, '.gitignore'), 'build/\n*.egg-info/\n__pycache__/\n');
    await fsp.writeFile(path.join(project, 'src', module, '__init__.py'), 'VALUE = 1\n');
    await fsp.writeFile(
      path.join(project, 'pyproject.toml'),
      `[build-system]\nrequires = ["setuptools>=78"]\nbuild-backend = "setuptools.build_meta"\n` +
        `[project]\nname = "${distribution}"\nversion = "${version}"\nrequires-python = ">=3.12,<3.13"\n` +
        `dependencies = [${dependencies}]\n[tool.setuptools.packages.find]\nwhere = ["src"]\n`,
    );
    git(repository, 'init');
    git(repository, 'config', 'core.autocrlf', 'false');
    git(repository, 'add', '.');
    git(
      repository,
      '-c',
      'user.name=Fixture',
      '-c',
      'user.email=fixture@example.invalid',
      'commit',
      '-m',
      'fixture',
    );
    return { repository, project, revision: git(repository, 'rev-parse', 'HEAD'), module };
  };
  try {
    const contract = await fixture('contract', 'VOXSCRIPT_CONTRACT', 'voxscript-contract', '0.2.0');
    const tts = await fixture(
      'tts',
      'PUZA_TTS',
      'puza-tts',
      '0.3.0',
      '"voxscript-contract>=0.2.0,<0.3.0"',
    );
    const wheels = path.join(base, 'wheels');
    execFileSync(
      uv,
      [
        'build',
        '--offline',
        '--wheel',
        '--no-build-isolation',
        '--python',
        python,
        '--out-dir',
        wheels,
        tts.project,
      ],
      { env, windowsHide: true, stdio: 'pipe' },
    );
    const wheel = path.join(wheels, 'puza_tts-0.3.0-py3-none-any.whl');
    const wheelSha = await hash(wheel);
    const invoke = (changes = {}) => {
      const values = {
        Id: `fixture-${crypto.randomUUID()}`,
        Python: python,
        Uv: uv,
        ContractRoot: contract.repository,
        ContractCommit: contract.revision,
        TtsRoot: tts.repository,
        TtsCommit: tts.revision,
        TtsWheel: wheel,
        TtsWheelSha256: wheelSha,
        ...changes,
      };
      const destination = owned(storage, path.join(storage, 'wheels', values.Id));
      outputs.add(destination);
      const result = spawnSync(
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-ExecutionPolicy',
          'Bypass',
          '-File',
          path.join(root, 'tools/ui-e2e/build-components.ps1'),
          ...Object.entries(values).flatMap(([key, value]) => [`-${key}`, value]),
        ],
        { cwd: base, env, encoding: 'utf8', windowsHide: true, timeout: 120000 },
      );
      assert.ifError(result.error);
      return { ...result, text: result.stdout + result.stderr, destination, id: values.Id };
    };
    await t.test(
      'offline success binds receipt identities and rebuilds deterministically',
      async () => {
        const first = invoke();
        assert.equal(first.status, 0, first.text);
        const second = invoke();
        assert.equal(second.status, 0, second.text);
        const receipt = JSON.parse(
          await fsp.readFile(path.join(first.destination, 'components.json')),
        );
        const other = JSON.parse(
          await fsp.readFile(path.join(second.destination, 'components.json')),
        );
        assert.equal(receipt.contract_commit, contract.revision);
        assert.equal(receipt.tts_commit, tts.revision);
        assert.equal(receipt.models_loaded, false);
        assert.equal(receipt.voices_modified, false);
        assert.equal(receipt.files.length, 2);
        assert.deepEqual(
          receipt.files.map((f) => f.sha256),
          other.files.map((f) => f.sha256),
        );
        for (const file of receipt.files) {
          assert.equal(file.sha256, await hash(owned(root, path.join(root, file.path))));
          assert.ok(file.path.startsWith(`.cache/ui-e2e/wheels/${first.id}/`));
        }
        const values = (value) =>
          value && typeof value === 'object'
            ? Object.values(value).flatMap(values)
            : typeof value === 'string'
              ? [value]
              : [];
        for (const value of values(receipt)) {
          assert.ok(!path.isAbsolute(value) && !path.win32.isAbsolute(value));
          assert.ok(!value.includes(base));
        }
        const repeat = invoke({ Id: first.id });
        assert.notEqual(repeat.status, 0);
        assert.match(repeat.text, /cannot be overwritten/);
        assert.deepEqual(
          JSON.parse(await fsp.readFile(path.join(first.destination, 'components.json'))),
          receipt,
        );
      },
    );
    await t.test(
      'wrong and missing source identities are rejected before a receipt exists',
      async () => {
        for (const values of [
          { ContractCommit: '0'.repeat(40) },
          { TtsCommit: 'short' },
          { ContractRoot: path.join(base, 'absent') },
          { TtsRoot: tts.project },
        ]) {
          const result = invoke(values);
          assert.notEqual(result.status, 0);
          await assert.rejects(fsp.access(path.join(result.destination, 'components.json')));
        }
      },
    );
    await t.test('dirty source and mismatched wheel hash cannot publish a receipt', async () => {
      const source = path.join(tts.project, 'src', tts.module, '__init__.py');
      await fsp.writeFile(source, 'VALUE = 2\n');
      const dirty = invoke();
      assert.notEqual(dirty.status, 0);
      assert.match(dirty.text, /must be clean/);
      await fsp.writeFile(source, 'VALUE = 1\n');
      const changed = invoke({ TtsWheelSha256: '0'.repeat(64) });
      assert.notEqual(changed.status, 0);
      assert.match(changed.text, /SHA-256 differs/);
      await assert.rejects(fsp.access(path.join(changed.destination, 'components.json')));
    });
    await t.test(
      'a matching caller hash cannot admit foreign metadata or changed package bytes',
      async () => {
        const mutate = [
          'import sys,zipfile',
          'source,target,mode=sys.argv[1:]',
          "with zipfile.ZipFile(source) as original,zipfile.ZipFile(target,'w') as changed:",
          ' for entry in original.infolist():',
          '  data=original.read(entry.filename)',
          "  if mode=='metadata' and entry.filename.endswith('/METADATA'): data=data.replace(b'Version: 0.3.0',b'Version: 9.0.0')",
          "  if mode=='payload' and entry.filename.endswith('/__init__.py'): data=b'VALUE = 9\\n'",
          "  if mode=='platform' and entry.filename.endswith('/WHEEL'): data=data.replace(b'Tag: py3-none-any',b'Tag: cp312-none-win_amd64')",
          "  if mode=='record' and entry.filename.endswith('/RECORD'): data=data.replace(b'sha256=',b'sha256=wrong',1)",
          '  changed.writestr(entry,data)',
        ].join('\n');
        const errors = {
          metadata: /metadata identity differs/,
          payload: /package payload differs/,
          platform: /platform metadata differs/,
          record: /RECORD integrity differs/,
        };
        for (const mode of Object.keys(errors)) {
          const folder = path.join(base, mode);
          await fsp.mkdir(folder);
          const changed = path.join(folder, path.basename(wheel));
          execFileSync(python, ['-I', '-B', '-c', mutate, wheel, changed, mode], {
            env,
            windowsHide: true,
          });
          const result = invoke({ TtsWheel: changed, TtsWheelSha256: await hash(changed) });
          assert.notEqual(result.status, 0);
          assert.match(result.text, errors[mode]);
          await assert.rejects(fsp.access(path.join(result.destination, 'components.json')));
        }
      },
    );
    await t.test('incompatible CONTRACT cannot gain a successful component receipt', async () => {
      const wrong = await fixture(
        'foreign-contract',
        'VOXSCRIPT_CONTRACT',
        'voxscript-contract',
        '0.3.0',
      );
      const result = invoke({ ContractRoot: wrong.repository, ContractCommit: wrong.revision });
      assert.notEqual(result.status, 0);
      assert.match(result.text, /does not satisfy/);
      await assert.rejects(fsp.access(path.join(result.destination, 'components.json')));
    });
    await t.test(
      'foreign build backends are refused before execution or receipt publication',
      async () => {
        const foreign = await fixture(
          'foreign-backend',
          'VOXSCRIPT_CONTRACT',
          'voxscript-contract',
          '0.2.0',
        );
        const file = path.join(foreign.project, 'pyproject.toml');
        const original = await fsp.readFile(file, 'utf8');
        await fsp.writeFile(file, original.replace('setuptools.build_meta', 'foreign_backend'));
        git(foreign.repository, 'add', '.');
        git(
          foreign.repository,
          '-c',
          'user.name=Fixture',
          '-c',
          'user.email=fixture@example.invalid',
          'commit',
          '-m',
          'fixture',
        );
        const result = invoke({
          ContractRoot: foreign.repository,
          ContractCommit: git(foreign.repository, 'rev-parse', 'HEAD'),
        });
        assert.notEqual(result.status, 0);
        assert.match(result.text, /static setuptools/);
        await assert.rejects(fsp.access(path.join(result.destination, 'components.json')));
      },
    );
    await t.test('ignored checkout payload cannot claim the pinned Git revision', async () => {
      const ignored = path.join(tts.project, 'src', tts.module, 'ignored_extra.py');
      const exclusion = path.join(base, 'fixture-excludes.txt');
      await fsp.writeFile(exclusion, 'ignored_extra.py\n');
      git(tts.repository, 'config', 'core.excludesfile', exclusion);
      try {
        await fsp.writeFile(ignored, 'EXTRA = True\n');
        assert.equal(git(tts.repository, 'status', '--porcelain'), '');
        const folder = path.join(base, 'ignored-payload');
        execFileSync(
          uv,
          [
            'build',
            '--offline',
            '--wheel',
            '--no-build-isolation',
            '--python',
            python,
            '--out-dir',
            folder,
            tts.project,
          ],
          { env, windowsHide: true, stdio: 'pipe' },
        );
        const forged = path.join(folder, path.basename(wheel));
        const result = invoke({ TtsWheel: forged, TtsWheelSha256: await hash(forged) });
        assert.notEqual(result.status, 0, 'Ignored source was accepted as the pinned commit');
        assert.match(result.text, /package payload differs/);
        await assert.rejects(fsp.access(path.join(result.destination, 'components.json')));
      } finally {
        await fsp.unlink(ignored);
        git(tts.repository, 'config', '--unset', 'core.excludesfile');
      }
    });
    assert.equal(git(contract.repository, 'status', '--porcelain'), '');
    assert.equal(git(tts.repository, 'status', '--porcelain'), '');
  } finally {
    for (const destination of outputs)
      await fsp.rm(owned(storage, destination), { recursive: true, force: true });
    await fsp.rm(owned(os.tmpdir(), base), { recursive: true, force: true });
  }
});
