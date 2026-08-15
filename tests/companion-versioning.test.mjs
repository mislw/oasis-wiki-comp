import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';


const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const packageJson = JSON.parse(read('../package.json'));
const tauriConfig = JSON.parse(read('../src-tauri/tauri.conf.json'));
const tauriBuildConfig = JSON.parse(read('../tauri.build.conf.json'));
const cargoToml = read('../src-tauri/Cargo.toml');
const cargoLock = read('../src-tauri/Cargo.lock');
const skillModule = read('../src-tauri/src/skill/mod.rs');
const settingsUi = read('../src/windows/Settings.tsx');
const bundledSkillVersion = read('../src-tauri/resources/skill/VERSION').trim();
const version = packageJson.version;


test('Companion uses the M.YYMMDD.N version format', () => {
  assert.match(version, /^\d+\.\d{6}\.\d+$/);
});


test('all Companion and bundled Skill version fields stay synchronized', () => {
  assert.equal(tauriConfig.version, version);
  assert.match(cargoToml, new RegExp(`^version = "${version.replaceAll('.', '\\.') }"$`, 'm'));
  assert.match(
    cargoLock,
    new RegExp(`name = "oasis-companion"\\r?\\nversion = "${version.replaceAll('.', '\\.') }"`),
  );
  assert.match(skillModule, new RegExp(`CURRENT_SKILL_VERSION: &str = "${version.replaceAll('.', '\\.') }"`));
  assert.match(settingsUi, new RegExp(`EXPECTED_VERSION = "${version.replaceAll('.', '\\.') }"`));
  assert.equal(bundledSkillVersion, version);
});


test('Windows MSI config uses numeric SemVer build metadata', () => {
  const [major, releaseDate, iteration] = version.split('.');
  const installerConfigVersion = `${major}.${releaseDate.slice(0, 2)}.${Number(releaseDate.slice(2))}+${iteration}`;
  assert.equal(tauriBuildConfig.version, installerConfigVersion);
});
