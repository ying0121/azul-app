import { useEffect } from 'react'
import { isDesktopApp } from '@/lib/tauri'

function isDevToolsShortcut(event: KeyboardEvent): boolean {
  const key = event.key

  if (key === 'F12') return true

  if (event.ctrlKey && event.shiftKey) {
    const lower = key.toLowerCase()
    return lower === 'i' || lower === 'j' || lower === 'c'
  }

  if (event.ctrlKey && key.toLowerCase() === 'u') return true

  return false
}

/** Disable context menu and devtools shortcuts in the Tauri desktop shell. */
export function useDesktopRestrictions() {
  useEffect(() => {
    if (!isDesktopApp()) return

    const blockContextMenu = (event: Event) => {
      event.preventDefault()
    }

    const blockDevTools = (event: KeyboardEvent) => {
      if (isDevToolsShortcut(event)) {
        event.preventDefault()
        event.stopPropagation()
      }
    }

    document.addEventListener('contextmenu', blockContextMenu)
    document.addEventListener('keydown', blockDevTools, true)

    return () => {
      document.removeEventListener('contextmenu', blockContextMenu)
      document.removeEventListener('keydown', blockDevTools, true)
    }
  }, [])
}
