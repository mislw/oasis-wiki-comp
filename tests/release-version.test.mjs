import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import {
  setReleaseVersion,
  toWindowsBuildVersion,
} from '../scripts/set-release-version.mjs';


const write = (root, path, content) => {
  const fullPath = join(root, path);
  mkdirSync(join(fullPath, '..'), { recursive: true });
  writeFileSync(fullPath, content, 'utf8');
};


test('maps the canonical release version to the Windows MSI version', () => {
  assert.equal(toWindowsBuildVersion('1.260817.6'), '1.26.817+6');
  assert.throws(() => toWindowsBuildVersion('1.26.817+6'), /M\.YYMMDD\.N/);
});


test('updates every Companion, bundled Skill, and plugin version field together', () => {
  const root = mkdtempSync(join(tmpdir(), 'oasis-version-'));
  write(root, 'package.json', '{"version":"1.260817.5"}\n');
  write(root, 'package-lock.json', '{"version":"1.260817.5","packages":{"":{"version":"1.260817.5"}}}\n');
  write(root, 'src-tauri/tauri.conf.json', '{"version":"1.260817.5"}\n');
  write(root, 'tauri.build.conf.json', '{"version":"1.26.817+5"}\n');
  write(root, 'src-tauri/Cargo.toml', '[package]\nversion = "1.260817.5"\n');
  write(root, 'src-tauri/Cargo.lock', '[[package]]\nname = "oasis-companion"\nversion = "1.260817.5"\n');
  write(root, 'src-tauri/src/skill/mod.rs', 'pub const CURRENT_SKILL_VERSION: &str = "1.260817.5";\n');
  write(root, 'src/windows/Settings.tsx', 'const EXPECTED_VERSION = "1.260817.5";\n');
  write(root, 'src-tauri/resources/skill/VERSION', '1.260817.5\n');
  write(root, 'src-tauri/resources/skill/tests/test_companion_versioning.py', "self.assertEqual(VERSION, '1.260817.5')\n");
  write(root, 'skills/oasis-wiki/VERSION', '1.260817.5\n');
  write(root, 'skills/oasis-wiki/tests/test_companion_versioning.py', "self.assertEqual(VERSION, '1.260817.5')\n");
  write(root, 'tests/companion-versioning.test.mjs', "const expectedReleaseVersion = '1.260817.5';\n");
  write(root, '.codex-plugin/plugin.json', '{"version":"1.260817.5"}\n');

  setReleaseVersion(root, '1.260817.6');
  setReleaseVersion(root, '1.260817.6');

  assert.equal(JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).version, '1.260817.6');
  assert.equal(JSON.parse(readFileSync(join(root, 'package-lock.json'), 'utf8')).packages[''].version, '1.260817.6');
  assert.equal(JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8')).version, '1.260817.6');
  assert.equal(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8'), '{"version":"1.260817.6"}\n');
  assert.equal(JSON.parse(readFileSync(join(root, 'tauri.build.conf.json'), 'utf8')).version, '1.26.817+6');
  assert.match(readFileSync(join(root, 'src-tauri/Cargo.toml'), 'utf8'), /version = "1\.260817\.6"/);
  assert.match(readFileSync(join(root, 'src-tauri/Cargo.lock'), 'utf8'), /version = "1\.260817\.6"/);
  assert.match(readFileSync(join(root, 'src-tauri/src/skill/mod.rs'), 'utf8'), /"1\.260817\.6"/);
  assert.match(readFileSync(join(root, 'src/windows/Settings.tsx'), 'utf8'), /"1\.260817\.6"/);
  assert.equal(readFileSync(join(root, 'src-tauri/resources/skill/VERSION'), 'utf8'), '1.260817.6\n');
  assert.match(readFileSync(join(root, 'src-tauri/resources/skill/tests/test_companion_versioning.py'), 'utf8'), /'1\.260817\.6'/);
  assert.equal(readFileSync(join(root, 'skills/oasis-wiki/VERSION'), 'utf8'), '1.260817.6\n');
  assert.match(readFileSync(join(root, 'skills/oasis-wiki/tests/test_companion_versioning.py'), 'utf8'), /'1\.260817\.6'/);
  assert.match(readFileSync(join(root, 'tests/companion-versioning.test.mjs'), 'utf8'), /'1\.260817\.6'/);
  assert.equal(JSON.parse(readFileSync(join(root, '.codex-plugin/plugin.json'), 'utf8')).version, '1.260817.6');
});
