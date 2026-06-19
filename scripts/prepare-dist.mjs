import { execSync } from 'node:child_process'

if (process.platform === 'win32') {
  try {
    execSync('taskkill /F /IM "Daily Team Huddle.exe" /T', { stdio: 'ignore' })
    console.log('Stopped Daily Team Huddle.exe')
  } catch {
    // App was not running
  }
}
