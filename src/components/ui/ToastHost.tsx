import { X } from 'lucide-react'
import { useToastStore } from '@/stores/alertStore'

export function ToastHost() {
  const toasts = useToastStore((s) => s.toasts)
  const dismiss = useToastStore((s) => s.dismiss)

  if (toasts.length === 0) return null

  return (
    <div className="toast-host" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className={`toast toast--${toast.type}`} role="status">
          <div className="toast__body">
            {toast.message ? (
              <>
                <p className="toast__title">{toast.title}</p>
                <p className="toast__message">{toast.message}</p>
              </>
            ) : (
              <p className="toast__title">{toast.title}</p>
            )}
          </div>
          <button
            type="button"
            className="toast__close"
            onClick={() => dismiss(toast.id)}
            aria-label="Dismiss notification"
          >
            <X size={16} />
          </button>
        </div>
      ))}
    </div>
  )
}
