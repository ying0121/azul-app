import { create } from 'zustand'
import { showToast, type ToastType } from '@/lib/toastr'

export type AlertType = ToastType

interface AlertState {
  show: (type: AlertType, title: string, message: string) => void
}

export const useAlertStore = create<AlertState>(() => ({
  show: (type, title, message) => {
    showToast(type, title, message)
  },
}))
