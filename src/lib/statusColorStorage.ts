import type { HedisStatusMap, StatusColorEntry, StatusColorsByType } from '@/types/statusColor'
import { EMPTY_STATUS_COLORS_BY_TYPE } from '@/types/statusColor'

const STORAGE_KEY = 'dh_status_colors'

interface CachedStatusColors {
  map: HedisStatusMap
  byType: StatusColorsByType
}

function isStatusColorEntry(value: unknown): value is StatusColorEntry {
  if (typeof value !== 'object' || value == null) return false
  const entry = value as StatusColorEntry
  return (
    typeof entry.display === 'string' &&
    typeof entry.status === 'string' &&
    typeof entry.description === 'string' &&
    typeof entry.text_color === 'string' &&
    typeof entry.bg_color === 'string'
  )
}

function parseCachedStatusColors(raw: string): CachedStatusColors | null {
  try {
    const parsed = JSON.parse(raw) as Partial<CachedStatusColors>
    if (typeof parsed.map !== 'object' || parsed.map == null) return null

    const map: HedisStatusMap = {}
    for (const [key, value] of Object.entries(parsed.map)) {
      const id = Number(key)
      if (!Number.isFinite(id) || !isStatusColorEntry(value)) continue
      map[id] = value
    }

    if (Object.keys(map).length === 0) return null

    return {
      map,
      byType: parsed.byType ?? EMPTY_STATUS_COLORS_BY_TYPE,
    }
  } catch {
    return null
  }
}

export function loadCachedStatusColors(): CachedStatusColors | null {
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return null
  return parseCachedStatusColors(raw)
}

export function saveCachedStatusColors(map: HedisStatusMap, byType: StatusColorsByType) {
  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      map,
      byType,
    }),
  )
}

export function clearCachedStatusColors() {
  localStorage.removeItem(STORAGE_KEY)
}
