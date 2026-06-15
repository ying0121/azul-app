export interface StatusColorEntry {
  display: string
  status: string
  description: string
  text_color: string
  bg_color: string
}

export type StatusColorType = 'measure' | 'patient' | 'eligibility' | 'measure_report'

export interface StatusColorItem extends StatusColorEntry {
  id: number
  type: StatusColorType
}

export type HedisStatusMap = Record<number, StatusColorEntry>

export type StatusColorsByType = Record<StatusColorType, StatusColorItem[]>

export const STATUS_COLOR_TABS: { id: StatusColorType; label: string }[] = [
  { id: 'measure', label: 'Measure' },
  { id: 'patient', label: 'Patient' },
  { id: 'eligibility', label: 'Eligibility' },
  { id: 'measure_report', label: 'Measure Report' },
]

export const EMPTY_STATUS_COLORS_BY_TYPE: StatusColorsByType = {
  measure: [],
  patient: [],
  eligibility: [],
  measure_report: [],
}

export function groupStatusColorsByType(items: StatusColorItem[]): StatusColorsByType {
  const grouped: StatusColorsByType = {
    measure: [],
    patient: [],
    eligibility: [],
    measure_report: [],
  }

  items.forEach((item) => {
    grouped[item.type].push(item)
  })

  for (const type of STATUS_COLOR_TABS.map((tab) => tab.id)) {
    grouped[type].sort((a, b) => a.id - b.id)
  }

  return grouped
}

export const HEDIS_STATUS_DETAIL_KEYS = new Set([
  'v_status',
  'm_status',
  'p_status',
  'e_status',
  'r_status',
])

export function lookupHedisStatus(
  map: HedisStatusMap,
  value: string | number | null | undefined,
): StatusColorEntry | null {
  if (value == null || value === '') return null

  const id = typeof value === 'number' ? value : parseInt(String(value).trim(), 10)
  if (Number.isNaN(id)) return null

  return map[id] ?? null
}

const PATIENT_STATUS_ROW_COLOR_IDS = new Set([28, 13, 6, 8])
const MEASURE_STATUS_ROW_COLOR_IDS = new Set([1, 5, 18])

export interface RowStatusStyle {
  backgroundColor?: string
  color?: string
  title?: string
}

function parseStatusId(value: string | number | null | undefined): number | null {
  if (value == null || value === '') return null

  const id = typeof value === 'number' ? value : parseInt(String(value).trim(), 10)
  return Number.isNaN(id) ? null : id
}

function findStatusByMinId(map: HedisStatusMap, minId: number): StatusColorEntry | null {
  let match: StatusColorEntry | null = null
  let matchId = Infinity

  for (const key of Object.keys(map)) {
    const id = Number(key)
    if (Number.isNaN(id) || id < minId) continue
    if (id < matchId) {
      matchId = id
      match = map[id]
    }
  }

  return match
}

export function resolveRowStatusStyle(
  map: HedisStatusMap,
  details: {
    m_status?: string | number
    p_status?: string | number
    e_status?: string | number
  },
): RowStatusStyle {
  const m_status = parseStatusId(details.m_status)
  const p_status = parseStatusId(details.p_status)

  let backgroundColor: string | undefined
  let color: string | undefined
  let title: string | undefined

  if (p_status != null) {
    const patientEntry = map[p_status]
    if (patientEntry) {
      title = patientEntry.display || patientEntry.status || undefined
      if (PATIENT_STATUS_ROW_COLOR_IDS.has(p_status)) {
        backgroundColor = patientEntry.bg_color || undefined
        color = patientEntry.text_color || undefined
      }
    }
  }

  if (m_status != null && MEASURE_STATUS_ROW_COLOR_IDS.has(m_status)) {
    const measureEntry = findStatusByMinId(map, m_status)
    if (measureEntry) {
      backgroundColor = measureEntry.bg_color || undefined
      color = measureEntry.text_color || undefined
    }
  }

  const result: RowStatusStyle = {}
  if (backgroundColor) result.backgroundColor = backgroundColor
  if (color) result.color = color
  if (title) result.title = title
  return result
}
