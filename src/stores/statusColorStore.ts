import { create } from 'zustand'
import { fetchStatusColors } from '@/api/statusColor'
import type { HedisStatusMap, StatusColorsByType } from '@/types/statusColor'
import { EMPTY_STATUS_COLORS_BY_TYPE } from '@/types/statusColor'

interface StatusColorState {
  hedisStatus: HedisStatusMap
  statusColorsByType: StatusColorsByType
  isLoading: boolean
  isLoaded: boolean
  loadStatusColors: () => Promise<void>
  reset: () => void
}

export const useStatusColorStore = create<StatusColorState>((set, get) => ({
  hedisStatus: {},
  statusColorsByType: EMPTY_STATUS_COLORS_BY_TYPE,
  isLoading: false,
  isLoaded: false,

  loadStatusColors: async () => {
    if (get().isLoading) return

    set({ isLoading: true })
    try {
      const data = await fetchStatusColors()
      set({
        hedisStatus: data.map,
        statusColorsByType: data.byType,
        isLoaded: true,
      })
    } finally {
      set({ isLoading: false })
    }
  },

  reset: () =>
    set({
      hedisStatus: {},
      statusColorsByType: EMPTY_STATUS_COLORS_BY_TYPE,
      isLoading: false,
      isLoaded: false,
    }),
}))
