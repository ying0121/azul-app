import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, '..')
const bundleDir = path.join(
  process.env.USERPROFILE ?? process.env.HOME ?? '',
  '.cargo',
  'daily-huddle-target',
  'release',
  'bundle',
)
const outDir = path.join(root, 'release-build')

function copyIfExists(src, dest) {
  if (!fs.existsSync(src)) return
  fs.mkdirSync(path.dirname(dest), { recursive: true })
  fs.copyFileSync(src, dest)
  console.log(`Copied ${path.basename(dest)}`)
}

fs.mkdirSync(outDir, { recursive: true })

copyIfExists(
  path.join(bundleDir, 'nsis', 'Daily Team Huddle_0.1.0_x64-setup.exe'),
  path.join(outDir, 'Daily Team Huddle Setup 0.1.0.exe'),
)
copyIfExists(
  path.join(bundleDir, 'msi', 'Daily Team Huddle_0.1.0_x64_en-US.msi'),
  path.join(outDir, 'Daily Team Huddle 0.1.0.msi'),
)

console.log(`Installers copied to ${path.relative(root, outDir)}`)
