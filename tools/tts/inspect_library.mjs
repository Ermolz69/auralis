import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import path from 'node:path';
const [root, title] = process.argv.slice(2);
assert.ok(root && title, 'Provide an application data root and the material name');
const database = new DatabaseSync(path.join(root, 'auralis.sqlite'), { readOnly: true });
try {
  const { user_version: schema } = database.prepare('PRAGMA user_version').get();
  assert.equal(schema, 18);
  const row = database.prepare('SELECT revision,payload FROM workspace_library WHERE id=1').get();
  const catalog = JSON.parse(row.payload);
  const asset = catalog.assets.find((a) => a.name === title);
  assert.ok(asset, 'The material must be persisted');
  assert.ok(catalog.standaloneOwners.includes(asset.ownerProjectId));
  const receipt = database
    .prepare('SELECT snapshot_sha256 FROM puza_contract_projects WHERE project_id=?')
    .get(asset.ownerProjectId);
  assert.match(receipt?.snapshot_sha256 ?? '', /^[a-f0-9]{64}$/);
  const workspace = catalog.workspaces.find((w) => w.assetId === asset.id);
  assert.ok(workspace?.tabs.some((t) => t.tool === 'text'));
  assert.ok(workspace.tabs.some((t) => t.tool === 'speech'));
  const drafts = workspace.tabs.map((t) => JSON.parse(t.draft));
  assert.ok(drafts.some((d) => typeof d.text === 'string' && d.text !== asset.text));
  console.log(
    JSON.stringify(
      {
        schema,
        revision: row.revision,
        assetId: asset.id,
        ownerProjectId: asset.ownerProjectId,
        contractSha256: receipt.snapshot_sha256,
        tools: workspace.tabs.map((t) => t.tool),
        distinctDraftPersisted: true,
      },
      null,
      2,
    ),
  );
} finally {
  database.close();
}
