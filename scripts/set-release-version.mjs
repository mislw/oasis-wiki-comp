import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';


const VERSION_PATTERN = /^(\d+)\.(\d{2})(\d{4})\.(\d+)$/;


export function toWindowsBuildVersion(version) {
  const match = VERSION_PATTERN.exec(version);
  if (!match) {
    throw new Error(`Version must use M.YYMMDD.N format: ${version}`);
  }
  const [, major, year, monthDay, iteration] = match;
  return `${major}.${year}.${Number(monthDay)}+${iteration}`;
}


function replaceJsonVersions(path, version, limit = 1) {
  const source = readFileSync(path, 'utf8');
  let replaced = 0;
  const updated = source.replace(/("version"\s*:\s*")[^"]+(")/g, (match, prefix, suffix) => {
    if (replaced >= limit) return match;
    replaced += 1;
    return `${prefix}${version}${suffix}`;
  });
  if (replaced !== limit) {
    throw new Error(`Expected ${limit} version field(s) in ${path}, found ${replaced}`);
  }
  writeFileSync(path, updated, 'utf8');
}


function replaceRequired(path, pattern, replacement) {
  const source = readFileSync(path, 'utf8');
  if (!pattern.test(source)) {
    throw new Error(`Version field not found in ${path}`);
  }
  const updated = source.replace(pattern, replacement);
  writeFileSync(path, updated, 'utf8');
}


export function setReleaseVersion(root, version) {
  const windowsVersion = toWindowsBuildVersion(version);

  replaceJsonVersions(join(root, 'package.json'), version);
  replaceJsonVersions(join(root, 'package-lock.json'), version, 2);
  replaceJsonVersions(join(root, 'src-tauri', 'tauri.conf.json'), version);
  replaceJsonVersions(join(root, 'tauri.build.conf.json'), windowsVersion);
  replaceJsonVersions(join(root, '.codex-plugin', 'plugin.json'), version);

  replaceRequired(
    join(root, 'src-tauri', 'Cargo.toml'),
    /^version = "[^"]+"$/m,
    `version = "${version}"`,
  );
  replaceRequired(
    join(root, 'src-tauri', 'Cargo.lock'),
    /(\[\[package\]\]\r?\nname = "oasis-companion"\r?\nversion = ")[^"]+("\r?\n)/,
    `$1${version}$2`,
  );
  replaceRequired(
    join(root, 'src-tauri', 'src', 'skill', 'mod.rs'),
    /CURRENT_SKILL_VERSION: &str = "[^"]+"/,
    `CURRENT_SKILL_VERSION: &str = "${version}"`,
  );
  replaceRequired(
    join(root, 'src', 'windows', 'Settings.tsx'),
    /EXPECTED_VERSION = "[^"]+"/,
    `EXPECTED_VERSION = "${version}"`,
  );

  writeFileSync(join(root, 'src-tauri', 'resources', 'skill', 'VERSION'), `${version}\n`, 'utf8');
  writeFileSync(join(root, 'skills', 'oasis-wiki', 'VERSION'), `${version}\n`, 'utf8');
  for (const testPath of [
    join(root, 'src-tauri', 'resources', 'skill', 'tests', 'test_companion_versioning.py'),
    join(root, 'skills', 'oasis-wiki', 'tests', 'test_companion_versioning.py'),
  ]) {
    replaceRequired(
      testPath,
      /self\.assertEqual\(VERSION, '[^']+'\)/,
      `self.assertEqual(VERSION, '${version}')`,
    );
  }
  replaceRequired(
    join(root, 'tests', 'companion-versioning.test.mjs'),
    /expectedReleaseVersion = '[^']+';/,
    `expectedReleaseVersion = '${version}';`,
  );
}


const isCli = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (isCli) {
  const version = process.argv[2];
  if (!version) {
    throw new Error('Usage: node scripts/set-release-version.mjs M.YYMMDD.N');
  }
  const root = join(dirname(fileURLToPath(import.meta.url)), '..');
  setReleaseVersion(root, version);
  console.log(`Release version synchronized: ${version} (${toWindowsBuildVersion(version)})`);
}
