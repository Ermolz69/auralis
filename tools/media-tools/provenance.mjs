import { targetAssets } from './manifest.mjs';

export function renderProvenance(manifest, target) {
  const assets = Object.fromEntries(
    targetAssets(manifest, target).map(({ name, url, sha256, output, tool }) => [
      name,
      {
        version: tool.version,
        output,
        sha256,
        downloadUrl: url,
        sourceUrl: tool.sourceUrl,
        sourceRevision: tool.sourceRevision,
        ...(tool.buildUrl
          ? { buildUrl: tool.buildUrl, buildRevision: tool.buildRevision }
          : {}),
        license: manifest.licenses[tool.license].spdx,
      },
    ]),
  );
  return `${JSON.stringify({ schemaVersion: 1, target, assets }, null, 2)}\n`;
}

export function renderNotices(manifest, target) {
  const ffmpeg = manifest.tools.ffmpeg;
  const ytdlp = manifest.tools['yt-dlp'];
  return `Auralis bundled media tools\n\nTarget: ${target}\n\nFFmpeg and ffprobe ${ffmpeg.version}\nLicense: ${manifest.licenses.ffmpeg.spdx}\nSource: ${ffmpeg.sourceUrl}\nSource revision: ${ffmpeg.sourceRevision}\nBuild scripts: ${ffmpeg.buildUrl}\nBuild revision: ${ffmpeg.buildRevision}\nFull license: ${manifest.licenses.ffmpeg.output}\n\nyt-dlp ${ytdlp.version}\nLicense: ${manifest.licenses['yt-dlp'].spdx}\nSource: ${ytdlp.sourceUrl}\nSource revision: ${ytdlp.sourceRevision}\nFull license: ${manifest.licenses['yt-dlp'].output}\n`;
}
