import fs from 'node:fs';
import path from 'node:path';

const repoRoot = path.resolve(import.meta.dirname, '..');
const packageJsonPath = path.join(repoRoot, 'package.json');
const tauriConfigPath = path.join(repoRoot, 'src-tauri', 'tauri.conf.json');
const cargoTomlPath = path.join(repoRoot, 'src-tauri', 'Cargo.toml');
const workspaceCargoTomlPath = path.join(repoRoot, 'Cargo.toml');

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
const version = packageJson.version;

if (!version) {
  throw new Error('package.json is missing a version field');
}

const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, 'utf8'));
tauriConfig.version = version;
fs.writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

const syncCargoVersion = (filePath, label) => {
  const source = fs.readFileSync(filePath, 'utf8');
  const next = source.replace(
    /^version = ".*"$/m,
    `version = "${version}"`,
  );

  if (source === next) {
    throw new Error(`Could not find a version field to update in ${label}`);
  }

  fs.writeFileSync(filePath, next);
};

syncCargoVersion(cargoTomlPath, 'src-tauri/Cargo.toml');
syncCargoVersion(workspaceCargoTomlPath, 'Cargo.toml');

console.log(`Synchronized application version to ${version}`);
