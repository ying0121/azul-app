import { execSync } from 'node:child_process'

if (process.platform === 'win32') {
  try {
    execSync('taskkill /F /IM "Daily Huddle.exe" /T', { stdio: 'ignore' })
    console.log('Stopped Daily Huddle.exe')
  } catch {
    // App was not running
  }
}
