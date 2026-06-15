import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

export function isDesktopApp(): boolean {
  return isTauri()
}

export function getAppWindow() {
  return getCurrentWindow()
}
