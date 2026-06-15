import { create } from 'zustand'
import { setAuthToken } from '@/api/client'
import {
  clearSession,
  getClinic,
  getToken,
  saveClinic,
  saveToken,
} from '@/lib/session'
import type { Clinic } from '@/types/auth'

interface AuthState {
  clinic: Clinic | null
  token: string | null
  isAuthenticated: boolean
  isLoading: boolean
  setClinic: (clinic: Clinic | null) => void
  setToken: (token: string | null) => void
  setAuthenticated: (value: boolean) => void
  setLoading: (value: boolean) => void
  setAuthSession: (clinic: Clinic, token: string) => void
  hydrateFromSession: () => void
  reset: () => void
}

export const useAuthStore = create<AuthState>((set) => ({
  clinic: null,
  token: null,
  isAuthenticated: false,
  isLoading: false,

  setClinic: (clinic) => set({ clinic }),
  setToken: (token) => {
    setAuthToken(token)
    set({ token })
  },
  setAuthenticated: (isAuthenticated) => set({ isAuthenticated }),
  setLoading: (isLoading) => set({ isLoading }),

  setAuthSession: (clinic, token) => {
    const trimmedToken = token.trim()
    setAuthToken(trimmedToken)
    saveToken(trimmedToken)
    saveClinic(clinic)
    set({
      clinic,
      token: trimmedToken,
      isAuthenticated: true,
    })
  },

  hydrateFromSession: () => {
    const token = getToken()
    const clinic = getClinic()

    if (token) {
      setAuthToken(token)
    }

    if (!token || clinic == null) {
      set({
        clinic: null,
        token: null,
        isAuthenticated: false,
      })
      return
    }

    set({
      clinic,
      token,
      isAuthenticated: true,
    })
  },

  reset: () => {
    setAuthToken(null)
    clearSession()
    set({
      clinic: null,
      token: null,
      isAuthenticated: false,
      isLoading: false,
    })
  },
}))
