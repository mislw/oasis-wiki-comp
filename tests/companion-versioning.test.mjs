import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
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
const expectedReleaseVersion = '1.260818.1';


test('Companion uses the approved August 18 release version', () => {
  assert.equal(version, expectedReleaseVersion);
});


test('Companion uses the M.YYMMDD.N version format', () => {
  assert.match(version, /^\d+\.\d{6}\.\d+$/);
});


test('MSI build config maps the canonical version to valid Windows SemVer', () => {
  const [, major, year, monthDay, iteration] = version.match(/^(\d+)\.(\d{2})(\d{4})\.(\d+)$/) ?? [];
  assert.equal(tauriBuildConfig.version, `${major}.${year}.${Number(monthDay)}+${iteration}`);
});


test('all Companion and bundled Skill version fields stay synchronized', () => {
  const pluginManifestPath = new URL('../.codex-plugin/plugin.json', import.meta.url);
  const pluginSkillVersionPath = new URL('../skills/oasis-wiki/VERSION', import.meta.url);
  assert.ok(existsSync(pluginManifestPath), '.codex-plugin/plugin.json must exist');
  assert.ok(existsSync(pluginSkillVersionPath), 'skills/oasis-wiki/VERSION must exist');
  const pluginManifest = JSON.parse(readFileSync(pluginManifestPath, 'utf8'));
  const pluginSkillVersion = readFileSync(pluginSkillVersionPath, 'utf8').trim();

  assert.equal(tauriConfig.version, version);
  assert.match(cargoToml, new RegExp(`^version = "${version.replaceAll('.', '\\.') }"$`, 'm'));
  assert.match(
    cargoLock,
    new RegExp(`name = "oasis-companion"\\r?\\nversion = "${version.replaceAll('.', '\\.') }"`),
  );
  assert.match(skillModule, new RegExp(`CURRENT_SKILL_VERSION: &str = "${version.replaceAll('.', '\\.') }"`));
  assert.match(settingsUi, new RegExp(`EXPECTED_VERSION = "${version.replaceAll('.', '\\.') }"`));
  assert.equal(bundledSkillVersion, version);
  assert.equal(pluginManifest.version, version);
  assert.equal(pluginSkillVersion, version);
});


test('Windows MSI config uses numeric SemVer build metadata', () => {
  const [major, releaseDate, iteration] = version.split('.');
  const installerConfigVersion = `${major}.${releaseDate.slice(0, 2)}.${Number(releaseDate.slice(2))}+${iteration}`;
  assert.equal(tauriBuildConfig.version, installerConfigVersion);
});


test('settings display the canonical Companion version instead of the MSI build mapping', () => {
  assert.match(settingsUi, /<span className="app-version">v\{EXPECTED_VERSION\}<\/span>/);
  assert.doesNotMatch(settingsUi, /<span className="app-version">v\{companionVersion\}<\/span>/);
});


test('settings identify the GitHub updater as Skill-only', () => {
  assert.match(settingsUi, /<h2>Skill 更新<\/h2>/);
  assert.match(settingsUi, /<span>Skill 仓库<\/span>/);
  assert.match(settingsUi, /这里只更新 oasis-wiki Skill，不会安装 Companion MSI/);
});
