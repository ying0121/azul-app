import {
  ALL_INSURANCES_ID,
  ALL_PCPS_ID,
  ALL_QUALITY_PROGRAM_ID,
  DEFAULT_APPT_FILTER,
  DEFAULT_SOURCE_FILTER,
  type ApptFilterState,
  type SavedDashboardFilters,
  type SourceFilterState,
} from '@/types/filters'

const STORAGE_KEY_PREFIX = 'dh_dashboard_filters_'

function storageKey(clinicId: string) {
  return `${STORAGE_KEY_PREFIX}${clinicId}`
}

export const DEFAULT_SAVED_FILTERS: SavedDashboardFilters = {
  insuranceId: ALL_INSURANCES_ID,
  qualityProgramId: ALL_QUALITY_PROGRAM_ID,
  pcpId: ALL_PCPS_ID,
  measureIds: [],
  sourceFilter: DEFAULT_SOURCE_FILTER,
  apptFilter: DEFAULT_APPT_FILTER,
}

function isApptFilterState(value: unknown): value is ApptFilterState {
  if (typeof value !== 'object' || value == null) return false
  const preset = (value as ApptFilterState).preset
  return (
    preset === 'today' ||
    preset === 'next_7' ||
    preset === 'next_15' ||
    preset === 'next_30' ||
    preset === 'custom'
  )
}

function isSourceFilterState(value: unknown): value is SourceFilterState {
  if (typeof value !== 'object' || value == null) return false
  const filter = value as SourceFilterState
  return (
    typeof filter.all === 'boolean' &&
    typeof filter.hedis === 'boolean' &&
    typeof filter.med_adh === 'boolean'
  )
}

function parseSavedFilters(raw: string): SavedDashboardFilters | null {
  try {
    const parsed = JSON.parse(raw) as Partial<SavedDashboardFilters>
    if (typeof parsed.insuranceId !== 'string') return null
    if (typeof parsed.qualityProgramId !== 'string') return null
    if (typeof parsed.pcpId !== 'string') return null
    if (!Array.isArray(parsed.measureIds) || !parsed.measureIds.every((id) => typeof id === 'string')) {
      return null
    }
    if (!isSourceFilterState(parsed.sourceFilter)) return null
    if (!isApptFilterState(parsed.apptFilter)) return null

    return {
      insuranceId: parsed.insuranceId,
      qualityProgramId: parsed.qualityProgramId,
      pcpId: parsed.pcpId,
      measureIds: parsed.measureIds,
      sourceFilter: parsed.sourceFilter,
      apptFilter: {
        preset: parsed.apptFilter.preset,
        customEndDate:
          typeof parsed.apptFilter.customEndDate === 'string'
            ? parsed.apptFilter.customEndDate
            : '',
      },
    }
  } catch {
    return null
  }
}

export function loadSavedFilters(clinicId: string): SavedDashboardFilters {
  const raw = localStorage.getItem(storageKey(clinicId))
  if (!raw) return { ...DEFAULT_SAVED_FILTERS }
  return parseSavedFilters(raw) ?? { ...DEFAULT_SAVED_FILTERS }
}

export function saveFilters(clinicId: string, filters: SavedDashboardFilters) {
  localStorage.setItem(storageKey(clinicId), JSON.stringify(filters))
}

export function clearSavedFilters(clinicId: string) {
  localStorage.removeItem(storageKey(clinicId))
}
