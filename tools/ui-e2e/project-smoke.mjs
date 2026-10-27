import assert from 'node:assert/strict';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { hash, json } from './core.mjs';
import { clickOnce, observeUntil } from './observe.mjs';
import { uiFailure } from './failures.mjs';

const title = 'UI metadata verification';
export async function createProject(page) {
  await page
    .getByRole('menubar', { name: 'Главное меню' })
    .getByRole('menuitem', { name: 'Файл', exact: true })
    .click();
  await page.getByRole('menuitem', { name: 'Новый проект…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Новый проект', exact: true });
  await dialog.getByLabel('Название', { exact: true }).fill(title);
  await clickOnce(dialog.getByRole('button', { name: 'Создать', exact: true }), () =>
    observeUntil(async () => {
      const error = dialog.getByRole('alert');
      if (await error.isVisible())
        throw uiFailure(await error.innerText(), 'Media project creation');
      return !(await dialog.isVisible());
    }, 120000),
  );
  await openProject(page);
}
export async function openProject(page) {
  await page
    .getByRole('navigation', { name: 'Библиотека и проекты', exact: true })
    .getByRole('button', { name: title, exact: true })
    .dblclick();
  await page.getByRole('region', { name: `Рабочее пространство ${title}`, exact: true }).waitFor();
  assert.equal(
    await page.getByRole('alert').count(),
    0,
    'Native media validation reported an error',
  );
}
export async function verifyProject(profile) {
  const database = new DatabaseSync(path.join(profile, 'auralis.sqlite'), { readOnly: true });
  let record, snapshot;
  try {
    assert.equal(database.prepare('PRAGMA user_version').get().user_version, 22);
    const projects = database.prepare('SELECT id,title FROM projects').all();
    assert.equal(projects.length, 1);
    assert.equal(projects[0].title, title);
    record = database
      .prepare('SELECT * FROM media_project_receipts WHERE project_id=?')
      .get(projects[0].id);
    assert.equal(record.state, 'ready');
    snapshot = JSON.parse(
      database
        .prepare('SELECT snapshot_json FROM puza_contract_projects WHERE project_id=?')
        .get(projects[0].id).snapshot_json,
    );
    assert.equal(snapshot.contract_version, '0.2.0');
    assert.equal(database.prepare('SELECT COUNT(*) AS n FROM jobs').get().n, 0);
    assert.equal(database.prepare('PRAGMA foreign_key_check').all().length, 0);
  } finally {
    database.close();
  }
  const manifest = path.join(profile, 'projects', record.project_id, 'auralis-project.json');
  assert.equal(await hash(manifest), record.manifest_sha256);
  const data = await json(manifest);
  assert.deepEqual(data, JSON.parse(record.manifest_json));
  assert.equal(data.project_id, record.project_id);
  const runtime = await json(path.join(profile, 'project-runtime.json'));
  assert.equal(runtime.package_version, '0.3.0');
  for (const file of runtime.files)
    assert.equal(await hash(path.join(runtime.package_root, file.path)), file.sha256);
  return {
    project_id: record.project_id,
    manifest_sha256: record.manifest_sha256,
    schema: 22,
    speech_contract: snapshot.contract_version,
    media_contract: runtime.package_version,
    media_package_files: runtime.files.length,
    model_inference: false,
    voices_modified: false,
  };
}
