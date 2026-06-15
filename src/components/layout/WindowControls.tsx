import { useCallback, useEffect, useState } from 'react'
import { Minus, Square, X } from 'lucide-react'
import { getAppWindow } from '@/lib/tauri'

export function WindowControls() {
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    const appWindow = getAppWindow()
    let disposed = false

    const syncMaximized = async () => {
      if (disposed) return
      setIsMaximized(await appWindow.isMaximized())
    }

    void syncMaximized()

    const unlisten = appWindow.onResized(() => {
      void syncMaximized()
    })

    return () => {
      disposed = true
      void unlisten.then((fn) => fn())
    }
  }, [])

  const minimize = useCallback(() => {
    void getAppWindow().minimize()
  }, [])

  const toggleMaximize = useCallback(() => {
    void getAppWindow().toggleMaximize()
  }, [])

  const close = useCallback(() => {
    void getAppWindow().close()
  }, [])

  return (
    <div className="window-controls">
      <button
        type="button"
        className="window-controls__btn"
        onClick={minimize}
        title="Minimize"
        aria-label="Minimize"
      >
        <Minus size={14} strokeWidth={2.25} />
      </button>
      <button
        type="button"
        className="window-controls__btn"
        onClick={toggleMaximize}
        title={isMaximized ? 'Restore' : 'Maximize'}
        aria-label={isMaximized ? 'Restore' : 'Maximize'}
      >
        <Square size={12} strokeWidth={2.25} />
      </button>
      <button
        type="button"
        className="window-controls__btn window-controls__btn--close"
        onClick={close}
        title="Close"
        aria-label="Close"
      >
        <X size={14} strokeWidth={2.25} />
      </button>
    </div>
  )
}
