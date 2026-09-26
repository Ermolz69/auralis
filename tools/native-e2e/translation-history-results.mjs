import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import path from 'node:path';

const LATER_SECOND_LINE = 'Поздняя правка второй реплики.';
const BRANCH_FIRST_LINE = 'Правка первой реплики из исходной версии.';

export async function verifyHistoryResults(dataRoot, translate, publications) {
  const outputs = [];
  const bytesByRevision = [];
  for (const [index, publication] of publications.entries()) {
    const result = translate
      .prepare('SELECT * FROM results WHERE result_id = ?')
      .get(publication.result_id);
    const root = path.resolve(dataRoot, 'projects');
    const file = path.resolve(root, ...publication.location_value.split('/'));
    if (!file.startsWith(`${root}${path.sep}`))
      throw new Error('History artifact escaped the managed root');
    const bytes = await fsp.readFile(file);
    if (
      !result ||
      result.run_id !== publication.run_id ||
      result.revision !== index + 1 ||
      result.revision !== publication.result_revision ||
      result.review_state !== 'needs_review' ||
      result.output_sha256 !== hash(bytes) ||
      Number(publication.size_bytes) !== bytes.length
    )
      throw new Error('Immutable history result or file differs across databases');
    const lines = bytes.toString('utf8').split('\n');
    if (
      lines.length !== 8 ||
      lines[0] !== '1' ||
      lines[1] !== '00:00:01,000 --> 00:00:02,000' ||
      lines[3] !== '' ||
      lines[4] !== '2' ||
      lines[5] !== '00:00:03,000 --> 00:00:04,000' ||
      lines[7] !== ''
    )
      throw new Error('Historical output changed protected SRT bytes');
    outputs.push({
      resultId: result.result_id,
      revision: result.revision,
      artifactId: publication.artifact_id,
      sha256: result.output_sha256,
    });
    bytesByRevision.push(bytes);
  }
  const modelLines = bytesByRevision[0].toString('utf8').split('\n');
  const laterLines = [...modelLines];
  laterLines[6] = LATER_SECOND_LINE;
  const branchLines = [...modelLines];
  branchLines[2] = BRANCH_FIRST_LINE;
  if (
    !bytesByRevision[1].equals(Buffer.from(laterLines.join('\n'))) ||
    !bytesByRevision[2].equals(Buffer.from(branchLines.join('\n')))
  )
    throw new Error('Historical branch merged later text or changed an untouched byte');
  if (new Set(outputs.map((output) => output.sha256)).size !== 3)
    throw new Error('Native history did not produce three distinct immutable outputs');
  return { outputs, modelLines: [modelLines[2], modelLines[6]] };
}

function hash(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
