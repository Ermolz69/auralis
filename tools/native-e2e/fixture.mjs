import childProcess from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { loadManifest, resolveTarget } from '../media-tools/manifest.mjs';

export async function prepareTranslationFixture(
  dataRoot,
  subtitlePath,
  translationCrashEnabled,
  translationVttEnabled,
  translationPackageEnabled,
  translationPauseEnabled = false,
) {
  const firstCue = '1\n00:00:01,000 --> 00:00:02,000\n你好。\n';
  const secondLine = translationPauseEnabled
    ? '我们先检查原来的字幕文件，再把需要翻译的文字取出来。翻译完成以后，程序会创建一个新的文件，保留原来的时间和顺序。如果中途停止，已经保存的部分不需要重新翻译。'
    : '再见。';
  const secondCue = `2\n00:00:03,000 --> 00:00:04,000\n${secondLine}\n`;
  const vttHeader = 'WEBVTT\r\n\r\nNOTE provenance\r\nnative fixture\r\n\r\n';
  const vttFirstCue = '00:00:01.000 --> 00:00:02.000\r\n你好。\r\n';
  const vttSecondCue = `\r\n00:00:03.000 --> 00:00:04.000\r\n${secondLine}\r\n`;
  const vttSource = `${vttHeader}${vttFirstCue}${translationCrashEnabled ? vttSecondCue : ''}`;
  await fsp.writeFile(
    subtitlePath,
    translationVttEnabled
      ? vttSource
      : translationCrashEnabled
        ? `${firstCue}\n${secondCue}`
        : firstCue,
  );
  if (translationPackageEnabled) return;
  const executable = process.env.AURALIS_TEST_LLAMA_SERVER;
  const modelFile = process.env.AURALIS_TEST_GGUF;
  if (!executable || !modelFile || !path.isAbsolute(executable) || !path.isAbsolute(modelFile)) {
    throw new Error(
      'Translation native E2E requires absolute AURALIS_TEST_LLAMA_SERVER and AURALIS_TEST_GGUF paths',
    );
  }
  for (const asset of [executable, modelFile]) {
    if (!(await fsp.stat(asset)).isFile())
      throw new Error(`Translation asset is not a file: ${asset}`);
  }
  await fsp.writeFile(
    path.join(dataRoot, 'translation-runtime.json'),
    JSON.stringify({
      executable,
      model_file: modelFile,
      context_tokens: 2048,
      gpu_layers: 99,
      startup_timeout_seconds: 120,
      readiness_interval_milliseconds: 250,
    }),
  );
}

export function prepareMediaTools(rootDir) {
  if (process.env.AURALIS_NATIVE_E2E_MEDIA_READY === '1') return;
  run('task', ['media:prepare'], rootDir, process.env, 600_000);
}

export function generateVideo(rootDir, sourcePath) {
  const manifest = loadManifest(path.join(rootDir, 'tools/media-tools/manifest.json'));
  const target = resolveTarget();
  const ffmpeg = path.join(rootDir, 'src-tauri/binaries', manifest.targets[target].ffmpeg.output);
  if (!fs.existsSync(ffmpeg)) throw new Error(`Prepared ffmpeg is missing: ${ffmpeg}`);

  run(
    ffmpeg,
    [
      '-hide_banner',
      '-loglevel',
      'error',
      '-f',
      'lavfi',
      '-i',
      'color=c=black:s=320x180:d=0.5',
      '-c:v',
      'libx264',
      '-pix_fmt',
      'yuv420p',
      '-an',
      '-y',
      sourcePath,
    ],
    rootDir,
    process.env,
    60_000,
  );
}

export async function startFixtureServer(sourcePath) {
  let requestCount = 0;
  const server = http.createServer((request, response) => {
    const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
    if (!['GET', 'HEAD'].includes(request.method ?? '') || requestUrl.pathname !== '/source.mp4') {
      response.writeHead(404, { connection: 'close' });
      response.end();
      return;
    }

    requestCount += 1;
    const size = fs.statSync(sourcePath).size;
    const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range ?? '');
    let start = 0;
    let end = size - 1;
    let status = 200;
    const headers = {
      'accept-ranges': 'bytes',
      'content-type': 'video/mp4',
    };

    if (range) {
      start = Number(range[1]);
      end = range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
      if (
        !Number.isSafeInteger(start) ||
        !Number.isSafeInteger(end) ||
        start > end ||
        start >= size
      ) {
        response.writeHead(416, { 'content-range': `bytes */${size}`, connection: 'close' });
        response.end();
        return;
      }
      status = 206;
      headers['content-range'] = `bytes ${start}-${end}/${size}`;
    }

    headers['content-length'] = String(end - start + 1);
    response.writeHead(status, headers);
    if (request.method === 'HEAD') {
      response.end();
      return;
    }
    const stream = fs.createReadStream(sourcePath, { start, end });
    stream.on('error', () => response.destroy());
    stream.pipe(response);
  });

  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  if (!address || typeof address === 'string') {
    server.close();
    throw new Error('Native E2E fixture server did not expose a TCP port');
  }

  return {
    url: `http://127.0.0.1:${address.port}/source.mp4`,
    get requestCount() {
      return requestCount;
    },
    async close() {
      server.closeAllConnections?.();
      await new Promise((resolve, reject) => {
        server.close((error) => (error ? reject(error) : resolve()));
      });
    },
  };
}

export function run(command, args, cwd, env, timeout) {
  const result = childProcess.spawnSync(command, args, {
    cwd,
    env,
    encoding: 'utf8',
    timeout,
    windowsHide: true,
    stdio: ['ignore', 'inherit', 'inherit'],
  });
  if (result.error || result.status !== 0) {
    throw new Error(`${command} failed: ${result.error?.message ?? `exit code ${result.status}`}`);
  }
}

export async function sha256(file) {
  return crypto
    .createHash('sha256')
    .update(await fsp.readFile(file))
    .digest('hex');
}
