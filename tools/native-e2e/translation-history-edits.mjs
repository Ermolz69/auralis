import crypto from 'node:crypto';

const LATER_SECOND_LINE = 'Поздняя правка второй реплики.';
const BRANCH_FIRST_LINE = 'Правка первой реплики из исходной версии.';

export function verifyHistoryEdits(host, translate, link, publications) {
  const rows = host
    .prepare(
      'SELECT * FROM translation_edit_intents WHERE translation_id = ? ORDER BY created_at, result_id',
    )
    .all(link.translation_id);
  if (rows.length !== 2) throw new Error('Native edits lost their metadata-only journal');
  const observations = [];
  for (const [index, text] of [LATER_SECOND_LINE, BRANCH_FIRST_LINE].entries()) {
    const publication = publications[index + 1];
    const intent = rows.find((row) => row.result_id === publication.result_id);
    const origin = translate
      .prepare('SELECT * FROM result_edit_provenance WHERE result_id = ?')
      .get(publication.result_id);
    const segmentId = index === 0 ? 2 : 1;
    const headId = publications[index].result_id;
    const baseId = publications[0].result_id;
    if (
      !intent ||
      !origin ||
      intent.project_id !== link.project_id ||
      intent.translation_id !== link.translation_id ||
      intent.run_id !== publication.run_id ||
      intent.base_result_id !== baseId ||
      intent.expected_head_result_id !== headId ||
      intent.expected_link_revision !== index + 2 ||
      publication.expected_link_revision !== intent.expected_link_revision ||
      origin.base_result_id !== baseId ||
      origin.observed_head_result_id !== headId ||
      origin.segment_id !== segmentId
    )
      throw new Error(
        'Native branch journal and provenance differ from the actual base/head request',
      );
    if (intent.request_sha256 !== requestDigest(intent, segmentId, [text]))
      throw new Error('Native branch request digest differs from the actual UI text');
    const selections = translate
      .prepare(
        'SELECT segment_id FROM result_edit_selections WHERE result_id = ? ORDER BY segment_id',
      )
      .all(publication.result_id);
    if (selections.length !== 1 || selections[0].segment_id !== segmentId)
      throw new Error('Historical branch inherited a later manual selection');
    observations.push({ intent, origin, selections });
  }
  return observations;
}

function requestDigest(intent, segmentId, lines) {
  const hash = crypto.createHash('sha256');
  const integer = (value) => {
    const bytes = Buffer.alloc(8);
    bytes.writeBigUInt64BE(BigInt(value));
    return bytes;
  };
  const append = (value) => {
    const bytes = Buffer.from(value, 'utf8');
    hash.update(integer(bytes.length));
    hash.update(bytes);
  };
  for (const value of [
    'auralis-historical-edit-v1',
    intent.project_id,
    intent.translation_id,
    intent.result_id,
    intent.base_result_id,
    intent.run_id,
    intent.expected_head_result_id,
  ])
    append(value);
  hash.update(integer(intent.expected_link_revision));
  const segment = Buffer.alloc(4);
  segment.writeUInt32BE(segmentId);
  hash.update(segment);
  hash.update(integer(lines.length));
  for (const line of lines) append(line);
  return hash.digest('hex');
}
