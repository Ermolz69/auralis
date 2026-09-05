import fs from 'node:fs';
import path from 'node:path';

export function verifyRepositoryPolicy(rootDir) {
  const taskfile = fs
    .readFileSync(path.join(rootDir, 'Taskfile.yml'), 'utf8')
    .replaceAll('\r\n', '\n');
  for (const task of ['desktop:before-dev', 'desktop:before-build']) {
    if (!taskfile.includes(`  ${task}:\n`)) throw new Error(`Taskfile is missing ${task}`);
    if (taskfile.includes(`  ${task}:\n    internal: true`)) {
      throw new Error(`${task} must be callable by the Tauri CLI`);
    }
  }

  const tauriConfig = JSON.parse(fs.readFileSync(path.join(rootDir, 'src-tauri/tauri.conf.json')));
  if (tauriConfig.bundle?.active !== true) {
    throw new Error('Tauri bundle must be active so release jobs produce installers');
  }
  const icons = tauriConfig.bundle?.icon ?? [];
  for (const icon of ['icons/icon.ico', 'icons/icon.icns', 'icons/128x128.png']) {
    if (!icons.includes(icon)) throw new Error(`Tauri bundle must declare ${icon}`);
    if (!fs.existsSync(path.join(rootDir, 'src-tauri', icon))) {
      throw new Error(`Tauri bundle icon is missing: ${icon}`);
    }
  }
  const resources = tauriConfig.bundle?.resources ?? [];
  const expectedResources = [
    'binaries/ffmpeg-*',
    'binaries/ffprobe-*',
    'binaries/yt-dlp-*',
    'binaries/FFMPEG-GPL-3.0.txt',
    'binaries/YT-DLP-UNLICENSE.txt',
    'binaries/THIRD-PARTY-NOTICES.txt',
    'binaries/media-tools-provenance.json',
  ];
  if (
    resources.length !== expectedResources.length ||
    !expectedResources.every((resource) => resources.includes(resource))
  ) {
    throw new Error('Tauri bundle media resources must match the audited allowlist');
  }
  if (tauriConfig.build?.beforeBuildCommand !== 'task desktop:before-build') {
    throw new Error('Tauri beforeBuildCommand must prepare media tools through Taskfile');
  }
  if (tauriConfig.build?.beforeDevCommand !== 'task desktop:before-dev') {
    throw new Error('Tauri beforeDevCommand must prepare media tools through Taskfile');
  }

  const ignored = fs.readFileSync(path.join(rootDir, '.gitignore'), 'utf8');
  for (const line of ['/src-tauri/binaries/*']) {
    if (!ignored.split(/\r?\n/).includes(line)) throw new Error(`Missing generated binary ignore: ${line}`);
  }

  for (const workflow of ['.github/workflows/tauri-build.yml', '.github/workflows/release.yml']) {
    const content = fs.readFileSync(path.join(rootDir, workflow), 'utf8');
    const prepareIndex = content.indexOf('run: task media:prepare');
    const buildIndex = content.indexOf('uses: tauri-apps/tauri-action@');
    const verifyIndex = content.indexOf('run: task media:bundle:verify');
    const smokeIndex = content.indexOf('run: task release:smoke:install-launch');
    if (prepareIndex < 0 || buildIndex < 0 || prepareIndex > buildIndex) {
      throw new Error(`${workflow} must prepare verified media tools before Tauri build`);
    }
    if (verifyIndex < buildIndex) {
      throw new Error(`${workflow} must verify the built bundle after Tauri build`);
    }
    if (smokeIndex < verifyIndex) {
      throw new Error(`${workflow} must launch the installed package after bundle verification`);
    }
  }
}
