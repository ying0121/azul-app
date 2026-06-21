import { formatUsDate, isNullishValue, normalizeDisplayValue } from '@/lib/formatDate'
import type { PatientRow } from '@/types/patient'
import { isHedisRow } from '@/types/patient'

function firstPresentDate(...values: Array<string | number | null | undefined>): string {
  for (const value of values) {
    if (isNullishValue(value)) continue
    const formatted = formatUsDate(value)
    if (formatted) return formatted
  }
  return ''
}

export function getRowApptDate(row: PatientRow): string {
  return firstPresentDate(row.details.appt_date, row.dos)
}

export function getRowDos(row: PatientRow): string {
  return formatUsDate(row.dos)
}

export function getRowValue1(row: PatientRow): string {
  if (!isHedisRow(row)) return ''
  return normalizeDisplayValue(row.details.value1)
}

export function getRowValue2(row: PatientRow): string {
  if (!isHedisRow(row)) return ''
  return normalizeDisplayValue(row.details.value2)
}

export function getRowRefillDue(row: PatientRow): string {
  if (isHedisRow(row)) return ''
  const value = row.details.refill_due || row.details.med1_refill_date
  return formatUsDate(value)
}

export function getRowCoverageEnds(row: PatientRow): string {
  if (isHedisRow(row)) return ''
  return formatUsDate(row.details.coverage_ends)
}
