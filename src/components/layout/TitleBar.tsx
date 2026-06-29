import { useEffect, useMemo, type MouseEvent } from 'react'
import { getAppWindow, isDesktopApp, startAppWindowDrag, toggleMaximizeAppWindow } from '@/lib/tauri'
import { useAuthStore } from '@/stores/authStore'
import { getClinicDisplayName } from '@/types/auth'
import { WindowControls } from '@/components/layout/WindowControls'

const APP_NAME = 'Daily Team Huddle'

function buildWindowTitle(
  clinic: ReturnType<typeof useAuthStore.getState>['clinic'],
  isAuthenticated: boolean,
): string {
  if (isAuthenticated && clinic != null) {
    return `${getClinicDisplayName(clinic)} | ${APP_NAME}`
  }
  return APP_NAME
}

export function TitleBar() {
  const clinic = useAuthStore((s) => s.clinic)
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

  const title = useMemo(
    () => buildWindowTitle(clinic, isAuthenticated),
    [clinic, isAuthenticated],
  )

  useEffect(() => {
    document.title = title
    if (isDesktopApp()) {
      void getAppWindow().setTitle(title)
    }
  }, [title])

  const handleDragMouseDown = (event: MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return

    if (event.detail === 2) {
      void toggleMaximizeAppWindow()
      return
    }

    void startAppWindowDrag()
  }

  return (
    <header className="titlebar">
      <div className="titlebar__drag" onMouseDown={handleDragMouseDown}>
        <span className="titlebar__title">{title}</span>
      </div>
      <WindowControls />
    </header>
  )
}
