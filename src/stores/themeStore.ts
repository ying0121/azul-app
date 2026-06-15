import { create } from 'zustand'

export type Theme = 'light' | 'dark'

const THEME_KEY = 'dh_theme'

function getSystemTheme(): Theme {
  if (typeof window === 'undefined') return 'dark'
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
}

function getStoredTheme(): Theme {
  const stored = localStorage.getItem(THEME_KEY)
  if (stored === 'light' || stored === 'dark') return stored
  return getSystemTheme()
}

function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme
}

interface ThemeState {
  theme: Theme
  setTheme: (theme: Theme) => void
  toggleTheme: () => void
  hydrateTheme: () => void
}

export const useThemeStore = create<ThemeState>((set, get) => ({
  theme: 'dark',

  setTheme: (theme) => {
    applyTheme(theme)
    localStorage.setItem(THEME_KEY, theme)
    set({ theme })
  },

  toggleTheme: () => {
    const next = get().theme === 'dark' ? 'light' : 'dark'
    get().setTheme(next)
  },

  hydrateTheme: () => {
    const theme = getStoredTheme()
    applyTheme(theme)
    set({ theme })
  },
}))
