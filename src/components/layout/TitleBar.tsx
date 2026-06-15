import { getAppWindow } from '@/lib/tauri'
import { WindowControls } from '@/components/layout/WindowControls'

const FAVICON_SRC = '/favicon.ico'

export function TitleBar() {
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
        <span className="titlebar__title">Daily Huddle</span>
      </div>
      <WindowControls />
    </header>
  )
}
