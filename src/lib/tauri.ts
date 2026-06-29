import { invoke, isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

export function isDesktopApp(): boolean {
  return isTauri()
}

export function getAppWindow() {
  return getCurrentWindow()
}

export async function minimizeAppWindow(): Promise<void> {
  await invoke('window_minimize')
}

export async function toggleMaximizeAppWindow(): Promise<void> {
  await invoke('window_toggle_maximize')
}

export async function hideAppWindow(): Promise<void> {
  await invoke('window_hide')
}

export async function isAppWindowMaximized(): Promise<boolean> {
  return invoke<boolean>('window_is_maximized')
}

export async function startAppWindowDrag(): Promise<void> {
  await getAppWindow().startDragging()
}
