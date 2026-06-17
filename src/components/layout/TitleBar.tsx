import { useEffect, useMemo } from 'react'
import { getAppWindow, isDesktopApp } from '@/lib/tauri'
import { useAuthStore } from '@/stores/authStore'
import { getClinicDisplayName } from '@/types/auth'
import { WindowControls } from '@/components/layout/WindowControls'

const FAVICON_SRC = '/favicon.ico'
const APP_NAME = 'Daily Huddle'

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

  const handleDoubleClick = () => {
    void getAppWindow().toggleMaximize()
  }

  return (
    <header className="titlebar">
      <div
        className="titlebar__drag"
        data-tauri-drag-region
        onDoubleClick={handleDoubleClick}
      >
        <img src={FAVICON_SRC} alt="" className="titlebar__icon" aria-hidden />
        <span className="titlebar__title">{title}</span>
      </div>
      <WindowControls />
    </header>
  )
}
