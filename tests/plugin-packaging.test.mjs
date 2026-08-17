import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';


const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const manifestPath = path.join(root, '.codex-plugin', 'plugin.json');
const bundledSkill = path.join(root, 'src-tauri', 'resources', 'skill');
const pluginSkill = path.join(root, 'skills', 'oasis-wiki');


function packagedFiles(directory, prefix = '') {
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.name === '__pycache__' || entry.name.endsWith('.pyc')) continue;
    const relative = path.join(prefix, entry.name);
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...packagedFiles(absolute, relative));
    } else {
      files.push(relative.replaceAll('\\', '/'));
    }
  }
  return files.sort();
}


test('repository is a Codex plugin that exposes the Oasis Wiki Skill', () => {
  assert.ok(existsSync(manifestPath), '.codex-plugin/plugin.json must exist');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const packageJson = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));

  assert.equal(manifest.name, 'oasis-wiki-comp');
  assert.equal(manifest.version, packageJson.version);
  assert.equal(manifest.skills, './skills/');
  assert.ok(existsSync(path.join(pluginSkill, 'SKILL.md')));
});


test('plugin Skill and Tauri bundled Skill contain identical packaged files', () => {
  assert.ok(existsSync(pluginSkill), 'skills/oasis-wiki must exist');
  assert.deepEqual(packagedFiles(pluginSkill), packagedFiles(bundledSkill));

  for (const relative of packagedFiles(bundledSkill)) {
    assert.deepEqual(
      readFileSync(path.join(pluginSkill, relative)),
      readFileSync(path.join(bundledSkill, relative)),
      `${relative} differs between plugin and Tauri resources`,
    );
  }
});


test('reinstall confirmation does not speculate about a minimal stub', () => {
  const settings = readFileSync(path.join(root, 'src', 'windows', 'Settings.tsx'), 'utf8');
  assert.doesNotMatch(settings, /minimal stub|最小\s*stub/iu);
});
