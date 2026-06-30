import { create } from 'zustand'
import { fetchStatusColors } from '@/api/statusColor'
import {
  loadCachedStatusColors,
  saveCachedStatusColors,
} from '@/lib/statusColorStorage'
import type { HedisStatusMap, StatusColorsByType } from '@/types/statusColor'
import { EMPTY_STATUS_COLORS_BY_TYPE } from '@/types/statusColor'

interface StatusColorState {
  hedisStatus: HedisStatusMap
  statusColorsByType: StatusColorsByType
  isLoading: boolean
  isLoaded: boolean
  loadStatusColors: (options?: { force?: boolean }) => Promise<void>
  reset: () => void
}

const initialCache = loadCachedStatusColors()

function applyCache(set: (partial: Partial<StatusColorState>) => void) {
  const cached = loadCachedStatusColors()
  if (!cached) return false

  set({
    hedisStatus: cached.map,
    statusColorsByType: cached.byType,
    isLoaded: true,
  })
  return true
}

export const useStatusColorStore = create<StatusColorState>((set, get) => ({
  hedisStatus: initialCache?.map ?? {},
  statusColorsByType: initialCache?.byType ?? EMPTY_STATUS_COLORS_BY_TYPE,
  isLoading: false,
  isLoaded: Boolean(initialCache),

  loadStatusColors: async (options) => {
    const force = options?.force ?? false
    if (get().isLoading) return
    if (get().isLoaded && !force) return

    set({ isLoading: true })
    try {
      const data = await fetchStatusColors()
      saveCachedStatusColors(data.map, data.byType)
      set({
        hedisStatus: data.map,
        statusColorsByType: data.byType,
        isLoaded: true,
      })
    } catch {
      applyCache(set)
    } finally {
      set({ isLoading: false })
    }
  },

  reset: () => {
    if (applyCache(set)) {
      set({ isLoading: false })
      return
    }

    set({
      hedisStatus: {},
      statusColorsByType: EMPTY_STATUS_COLORS_BY_TYPE,
      isLoading: false,
      isLoaded: false,
    })
  },
}))
