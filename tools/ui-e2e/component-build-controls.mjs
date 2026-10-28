import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

export async function staticBuildControls({
  t,
  base,
  python,
  uv,
  env,
  fixture,
  invoke,
  hash,
  git,
  wheel,
}) {
  const commit = (repository) => {
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
    return git(repository, 'rev-parse', 'HEAD');
  };
  const build = (source, folder) => {
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
        source.project,
      ],
      { env, windowsHide: true, stdio: 'pipe' },
    );
    return path.join(folder, path.basename(wheel));
  };
  const reject = async (values, message) => {
    const result = invoke(values);
    assert.notEqual(result.status, 0, result.text);
    assert.match(result.text, message);
    await assert.rejects(fsp.access(path.join(result.destination, 'components.json')));
    return result;
  };
  const entryWheel = async (source, label, entries) => {
    const folder = path.join(base, label);
    await fsp.mkdir(folder);
    const target = path.join(folder, path.basename(wheel));
    const change = [
      'import base64,csv,hashlib,io,sys,zipfile',
      'source,target,entries=sys.argv[1:]',
      'with zipfile.ZipFile(source) as original: files={name:original.read(name) for name in original.namelist()}',
      "record=next(name for name in files if name.endswith('.dist-info/RECORD'))",
      "files[record.removesuffix('RECORD')+'entry_points.txt']=entries.encode()",
      'rows=[]',
      'for name,data in files.items():',
      " rows.append([name,'',''] if name==record else [name,'sha256='+base64.urlsafe_b64encode(hashlib.sha256(data).digest()).decode().rstrip('='),str(len(data))])",
      'stream=io.StringIO();csv.writer(stream).writerows(rows);files[record]=stream.getvalue().encode()',
      "with zipfile.ZipFile(target,'w') as output:",
      ' for name,data in files.items(): output.writestr(name,data)',
    ].join('\n');
    execFileSync(python, ['-I', '-B', '-c', change, source, target, entries], {
      env,
      windowsHide: true,
    });
    return { TtsWheel: target, TtsWheelSha256: await hash(target) };
  };
  await t.test('static pyproject policy rejects alternate build execution before uv', async () => {
    for (const mode of ['backend-path', 'cmdclass', 'setup.cfg']) {
      const source = await fixture(
        `static-${mode}`,
        'VOXSCRIPT_CONTRACT',
        'voxscript-contract',
        '0.2.0',
      );
      const config = path.join(source.project, 'pyproject.toml');
      if (mode === 'backend-path') {
        const value = await fsp.readFile(config, 'utf8');
        await fsp.writeFile(
          config,
          value.replace(
            'build-backend = "setuptools.build_meta"',
            'build-backend = "setuptools.build_meta"\nbackend-path = ["backend"]',
          ),
        );
        const backend = path.join(source.project, 'backend/setuptools');
        await fsp.mkdir(backend, { recursive: true });
        await fsp.writeFile(path.join(backend, '__init__.py'), '');
        await fsp.writeFile(
          path.join(backend, 'build_meta.py'),
          "from pathlib import Path\nPath(__file__).parents[2].joinpath('fixture-executed.txt').write_text('backend')\nraise RuntimeError('fixture backend executed')\n",
        );
      } else if (mode === 'cmdclass') {
        await fsp.appendFile(
          config,
          '\n[tool.setuptools.cmdclass]\nbuild_py = "buildhook.CustomBuild"\n',
        );
        await fsp.writeFile(
          path.join(source.project, 'buildhook.py'),
          "from pathlib import Path\nfrom setuptools.command.build_py import build_py\nclass CustomBuild(build_py):\n def run(self):\n  Path(__file__).with_name('fixture-executed.txt').write_text('cmdclass')\n  super().run()\n",
        );
      } else {
        await fsp.writeFile(path.join(source.project, 'setup.cfg'), '[build_py]\nforce = 1\n');
      }
      const revision = commit(source.repository);
      const result = invoke({ ContractRoot: source.repository, ContractCommit: revision });
      await assert.rejects(
        fsp.access(path.join(result.destination, '.build/VOXSCRIPT_CONTRACT/fixture-executed.txt')),
        `Custom ${mode} code ran before static-policy rejection`,
      );
      assert.notEqual(result.status, 0, result.text);
      assert.match(result.text, /static setuptools/);
      await assert.rejects(fsp.access(path.join(result.destination, 'components.json')));
    }
  });
  await t.test(
    'no-script sources cannot admit honestly recorded console, GUI or plugin entry points',
    async () => {
      for (const [label, entries] of [
        ['console', '[console_scripts]\ninjected = puza_tts:main\n'],
        ['gui', '[gui_scripts]\ninjected = puza_tts:main\n'],
        ['plugin', '[auralis.plugins]\ninjected = puza_tts:main\n'],
      ]) {
        await reject(
          await entryWheel(wheel, `undeclared-${label}`, entries),
          /entry points differ/,
        );
      }
    },
  );
  await t.test(
    'declared scripts reject additional wheel groups even with a valid RECORD',
    async () => {
      const source = await fixture(
        'declared-script',
        'PUZA_TTS',
        'puza-tts',
        '0.3.0',
        '"voxscript-contract>=0.2.0,<0.3.0"',
        '\n[project.scripts]\ncommand = "puza_tts:main"\n',
      );
      const original = build(source, path.join(base, 'declared-script-wheel'));
      for (const [label, extra] of [
        ['gui', '[gui_scripts]\ninjected = puza_tts:main\n'],
        ['plugin', '[auralis.plugins]\ninjected = puza_tts:main\n'],
      ]) {
        await reject(
          {
            TtsRoot: source.repository,
            TtsCommit: source.revision,
            ...(await entryWheel(
              original,
              `declared-extra-${label}`,
              `[console_scripts]\ncommand = puza_tts:main\n${extra}`,
            )),
          },
          /entry points differ/,
        );
      }
    },
  );
  await t.test(
    'complete declared entry points preserve case-sensitive names and reject renamed keys',
    async () => {
      const source = await fixture(
        'complete-entrypoints',
        'PUZA_TTS',
        'puza-tts',
        '0.3.0',
        '"voxscript-contract>=0.2.0,<0.3.0"',
        '\n[project.scripts]\nCaseCommand = "puza_tts:main"\n[project.gui-scripts]\nCaseGui = "puza_tts:main"\n[project.entry-points."auralis.plugins"]\nCasePlugin = "puza_tts:main"\n',
      );
      const original = build(source, path.join(base, 'complete-entrypoints-wheel'));
      const positive = invoke({
        TtsRoot: source.repository,
        TtsCommit: source.revision,
        TtsWheel: original,
        TtsWheelSha256: await hash(original),
      });
      assert.equal(positive.status, 0, positive.text);
      const entries =
        '[console_scripts]\ncasecommand = puza_tts:main\n[gui_scripts]\nCaseGui = puza_tts:main\n[auralis.plugins]\nCasePlugin = puza_tts:main\n';
      await reject(
        {
          TtsRoot: source.repository,
          TtsCommit: source.revision,
          ...(await entryWheel(original, 'renamed-entrypoint', entries)),
        },
        /entry points differ/,
      );
    },
  );
}
