import toastr from 'toastr'

export type ToastType = 'error' | 'warning' | 'success' | 'info'

let configured = false

export function configureToastr() {
  if (configured) return
  configured = true

  toastr.options = {
    closeButton: true,
    debug: false,
    newestOnTop: true,
    progressBar: true,
    positionClass: 'toast-top-right',
    preventDuplicates: true,
    showDuration: 300,
    hideDuration: 300,
    timeOut: 5000,
    extendedTimeOut: 1000,
    showEasing: 'swing',
    hideEasing: 'linear',
    showMethod: 'fadeIn',
    hideMethod: 'fadeOut',
  }
}

export function showToast(type: ToastType, title: string, message: string) {
  configureToastr()

  const body = message.trim() || title
  const toastTitle = message.trim() ? title : undefined

  switch (type) {
    case 'success':
      toastr.success(body, toastTitle)
      break
    case 'error':
      toastr.error(body, toastTitle)
      break
    case 'warning':
      toastr.warning(body, toastTitle)
      break
    case 'info':
      toastr.info(body, toastTitle)
      break
  }
}
