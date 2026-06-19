import { create } from 'zustand'

export type AlertType = 'error' | 'warning' | 'success' | 'info'

export interface ToastItem {
  id: number
  type: AlertType
  title: string
  message: string
}

interface AlertState {
  toasts: ToastItem[]
  show: (type: AlertType, title: string, message: string) => void
  dismiss: (id: number) => void
}

const TOAST_DURATION_MS = 5000

export const useAlertStore = create<AlertState>((set, get) => ({
  toasts: [],
  show: (type, title, message) => {
    const id = Date.now() + Math.floor(Math.random() * 1000)
    set((state) => ({
      toasts: [...state.toasts, { id, type, title, message }],
    }))
    window.setTimeout(() => {
      get().dismiss(id)
    }, TOAST_DURATION_MS)
  },
  dismiss: (id) => {
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== id),
    }))
  },
}))

export const useToastStore = useAlertStore
