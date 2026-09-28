import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';


const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (relative) => readFileSync(path.join(root, relative), 'utf8');


test('the unified repository owns the canonical Skill and update flow', () => {
  const schema = read('src-tauri/src/config/schema.rs');
  const settings = read('src/windows/Settings.tsx');
  const syncScript = read('scripts/sync-bundled-skill.ps1');
  const readme = read('README.md');

  assert.match(schema, /"mislw\/oasis-wiki-comp"\.into\(\)/);
  assert.match(settings, /placeholder="mislw\/oasis-wiki-comp"/);
  assert.match(syncScript, /\.\.\\skills\\oasis-wiki/);
  assert.doesNotMatch(syncScript, /SkillRepository|\.\.\\\.\.\\oasis-wiki/);
  assert.match(readme, /canonical Skill source lives in `skills\/oasis-wiki`/);
  assert.match(readme, /single repository/i);
});
