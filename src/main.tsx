import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './jquery-shim'
import 'toastr/build/toastr.min.css'
import './index.css'
import { configureToastr } from '@/lib/toastr'
import App from './App.tsx'

configureToastr()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
