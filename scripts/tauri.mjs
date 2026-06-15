import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, '..')
const cargoBin = path.join(os.homedir(), '.cargo', 'bin')
const cargoExe = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
const cargoPath = path.join(cargoBin, cargoExe)

if (!fs.existsSync(cargoPath)) {
  console.error('')
  console.error('Rust is required for Tauri but cargo was not found.')
  console.error('Install Rust from https://rustup.rs/ then restart your terminal.')
  console.error('')
  process.exit(1)
}

const pathKey = process.platform === 'win32' ? 'Path' : 'PATH'
const separator = process.platform === 'win32' ? ';' : ':'
const env = {
  ...process.env,
  [pathKey]: `${cargoBin}${separator}${process.env[pathKey] ?? ''}`,
}

const args = process.argv.slice(2)
const tauriCli = path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js')
const result = spawnSync(process.execPath, [tauriCli, ...args], {
  cwd: root,
  stdio: 'inherit',
  env,
})

process.exit(result.status ?? 1)
