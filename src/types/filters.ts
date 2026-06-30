import {
  addCalendarDays,
  getClinicTodayDateString,
  getClinicYear,
} from '@/lib/clinicDate'

export const ALL_INSURANCES_ID = '0'
export const ALL_QUALITY_PROGRAM_ID = '0'
export const ALL_PCPS_ID = '0'

export interface InsuranceOption {
  ins_id: string
  ins_name: string
}

export interface QualityProgramOption {
  qp_id: string
  qp_name: string
}

export interface PcpOption {
  pcp_id: string
  pcp_name: string
}

export interface MeasureOption {
  measure_id: string
  measure: string
}

export const ALL_MEASURES_PLACEHOLDER = 'All Measures'

export const ALL_INSURANCES_OPTION: InsuranceOption = {
  ins_id: ALL_INSURANCES_ID,
  ins_name: 'All Insurances',
}

export const ALL_QUALITY_PROGRAM_OPTION: QualityProgramOption = {
  qp_id: ALL_QUALITY_PROGRAM_ID,
  qp_name: 'All Quality Program',
}

export const ALL_PCPS_OPTION: PcpOption = {
  pcp_id: ALL_PCPS_ID,
  pcp_name: 'All PCPs',
}

export type ApptFilterPreset = 'today' | 'next_7' | 'next_15' | 'next_30' | 'custom'

export interface ApptFilterState {
  preset: ApptFilterPreset
  customEndDate: string
}

export interface PatientFilters {
  clinic_id?: string | number
  ins_id?: string
  qp_id?: string
  pcp_id?: string
  cyear?: string | number
  filter?: string
  measures?: string
  appt_start?: string
  appt_end?: string
}

export const DEFAULT_APPT_FILTER: ApptFilterState = {
  preset: 'today',
  customEndDate: '',
}

export interface SourceFilterState {
  all: boolean
  hedis: boolean
  med_adh: boolean
}

export interface SavedDashboardFilters {
  insuranceId: string
  qualityProgramId: string
  pcpId: string
  measureIds: string[]
  sourceFilter: SourceFilterState
  apptFilter: ApptFilterState
}

export function matchesMeasureFilter(measureId: string, selectedIds: string[]): boolean {
  if (selectedIds.length === 0) return true
  return selectedIds.includes(measureId)
}

export function isDefaultMeasureFilter(selectedIds: string[]): boolean {
  return selectedIds.length === 0
}

export function formatMeasuresParam(selectedIds: string[]): string {
  return selectedIds.filter(Boolean).join(',')
}

export const DEFAULT_SOURCE_FILTER: SourceFilterState = {
  all: true,
  hedis: false,
  med_adh: false,
}

export function matchesSourceFilter(
  source: 'hedis' | 'med_adh',
  filter: SourceFilterState,
): boolean {
  if (filter.all) return true
  return source === 'hedis' ? filter.hedis : filter.med_adh
}

export function isDefaultSourceFilter(filter: SourceFilterState): boolean {
  return filter.all && !filter.hedis && !filter.med_adh
}

export function toggleSourceFilterAll(): SourceFilterState {
  return { ...DEFAULT_SOURCE_FILTER }
}

export function toggleSourceFilterHedis(current: SourceFilterState): SourceFilterState {
  if (current.all) {
    return { all: false, hedis: true, med_adh: false }
  }

  const hedis = !current.hedis
  const med_adh = current.med_adh

  if (!hedis && !med_adh) return { ...DEFAULT_SOURCE_FILTER }
  if (hedis && med_adh) return { ...DEFAULT_SOURCE_FILTER }
  return { all: false, hedis, med_adh }
}

export function toggleSourceFilterMedAdh(current: SourceFilterState): SourceFilterState {
  if (current.all) {
    return { all: false, hedis: false, med_adh: true }
  }

  const med_adh = !current.med_adh
  const hedis = current.hedis

  if (!hedis && !med_adh) return { ...DEFAULT_SOURCE_FILTER }
  if (hedis && med_adh) return { ...DEFAULT_SOURCE_FILTER }
  return { all: false, hedis, med_adh }
}

export const APPT_FILTER_OPTIONS: { value: ApptFilterPreset; label: string }[] = [
  { value: 'today', label: 'Today' },
  { value: 'next_7', label: 'Next 7 Days' },
  { value: 'next_15', label: 'Next 15 Days' },
  { value: 'next_30', label: 'Next 30 Days' },
  { value: 'custom', label: 'Custom Selection' },
]

export function getTodayDateString(): string {
  return getClinicTodayDateString()
}

export function resolveApptRange(state: ApptFilterState): {
  appt_start: string
  appt_end: string
} {
  const appt_start = getClinicTodayDateString()

  switch (state.preset) {
    case 'today':
      return { appt_start, appt_end: appt_start }
    case 'next_7':
      return { appt_start, appt_end: addCalendarDays(appt_start, 7) }
    case 'next_15':
      return { appt_start, appt_end: addCalendarDays(appt_start, 15) }
    case 'next_30':
      return { appt_start, appt_end: addCalendarDays(appt_start, 30) }
    case 'custom': {
      const end = state.customEndDate && state.customEndDate >= appt_start
        ? state.customEndDate
        : appt_start
      return { appt_start, appt_end: end }
    }
    default:
      return { appt_start, appt_end: appt_start }
  }
}

export function getCurrentYear(): number {
  return getClinicYear()
}

export function formatApptFilterLabel(state: ApptFilterState): string {
  if (state.preset === 'custom') {
    const end = state.customEndDate || getTodayDateString()
    return `Custom (${getTodayDateString()} – ${end})`
  }
  return APPT_FILTER_OPTIONS.find((option) => option.value === state.preset)?.label ?? 'Today'
}

export function formatSourceFilterLabel(filter: SourceFilterState): string {
  if (filter.all) return 'All Sources'
  const parts: string[] = []
  if (filter.hedis) parts.push('HEDIS')
  if (filter.med_adh) parts.push('Med Adh')
  return parts.length > 0 ? parts.join(', ') : 'All Sources'
}

export function formatMeasureFilterLabel(
  selectedIds: string[],
  options: MeasureOption[],
): string {
  if (selectedIds.length === 0) return ALL_MEASURES_PLACEHOLDER
  const labels = selectedIds
    .map((id) => options.find((option) => option.measure_id === id)?.measure ?? id)
    .filter(Boolean)
  return labels.length > 0 ? labels.join(', ') : ALL_MEASURES_PLACEHOLDER
}

export function formatFilterStatusLabel(input: {
  insuranceName: string
  qualityProgramName: string
  pcpName: string
  measureLabel: string
  sourceLabel: string
  apptLabel: string
}): string {
  return [
    input.insuranceName,
    input.qualityProgramName,
    input.pcpName,
    input.measureLabel,
    input.sourceLabel,
    input.apptLabel,
  ].join(' · ')
}

export function hasNonDefaultFilters(input: {
  insuranceId: string
  qualityProgramId: string
  pcpId: string
  measureIds: string[]
  sourceFilter: SourceFilterState
  apptFilter: ApptFilterState
}): boolean {
  return (
    input.insuranceId !== ALL_INSURANCES_ID ||
    input.qualityProgramId !== ALL_QUALITY_PROGRAM_ID ||
    input.pcpId !== ALL_PCPS_ID ||
    !isDefaultMeasureFilter(input.measureIds) ||
    !isDefaultSourceFilter(input.sourceFilter) ||
    input.apptFilter.preset !== DEFAULT_APPT_FILTER.preset ||
    (input.apptFilter.preset === 'custom' && input.apptFilter.customEndDate !== '')
  )
}
