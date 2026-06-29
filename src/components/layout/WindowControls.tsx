import { useCallback, useEffect, useState, type MouseEvent } from 'react'
import { Minus, Square, X } from 'lucide-react'
import {
  getAppWindow,
  hideAppWindow,
  isAppWindowMaximized,
  minimizeAppWindow,
  toggleMaximizeAppWindow,
} from '@/lib/tauri'

function stopTitlebarMouseDown(event: MouseEvent<HTMLButtonElement>) {
  event.preventDefault()
  event.stopPropagation()
}

export function WindowControls() {
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    let disposed = false

    const syncMaximized = async () => {
      if (disposed) return
      try {
        setIsMaximized(await isAppWindowMaximized())
      } catch {
        setIsMaximized(false)
      }
    }

    void syncMaximized()

    const unlisten = getAppWindow().onResized(() => {
      void syncMaximized()
    })

    return () => {
      disposed = true
      void unlisten.then((fn) => fn())
    }
  }, [])

  const minimize = useCallback(() => {
    void minimizeAppWindow()
  }, [])

  const toggleMaximize = useCallback(() => {
    void toggleMaximizeAppWindow().then(() => isAppWindowMaximized().then(setIsMaximized))
  }, [])

  const close = useCallback(() => {
    void hideAppWindow()
  }, [])

  return (
    <div className="window-controls">
      <button
        type="button"
        className="window-controls__btn"
        onMouseDown={stopTitlebarMouseDown}
        onClick={minimize}
        title="Minimize"
        aria-label="Minimize"
      >
        <Minus size={14} strokeWidth={2.25} />
      </button>
      <button
        type="button"
        className="window-controls__btn"
        onMouseDown={stopTitlebarMouseDown}
        onClick={toggleMaximize}
        title={isMaximized ? 'Restore' : 'Maximize'}
        aria-label={isMaximized ? 'Restore' : 'Maximize'}
      >
        <Square size={12} strokeWidth={2.25} />
      </button>
      <button
        type="button"
        className="window-controls__btn window-controls__btn--close"
        onMouseDown={stopTitlebarMouseDown}
        onClick={close}
        title="Close"
        aria-label="Close"
      >
        <X size={14} strokeWidth={2.25} />
      </button>
    </div>
  )
}
