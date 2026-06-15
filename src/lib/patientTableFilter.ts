import type { PatientRow } from '@/types/patient'
import { matchesSourceFilter, type SourceFilterState } from '@/types/filters'

export function matchesPatientSearch(row: PatientRow, search: string): boolean {
  const filterValue = search.trim().toLowerCase()
  if (!filterValue) return true

  const haystack = [
    row.pt_fname,
    row.pt_lname,
    row.pt_subno,
    row.pt_emr_id,
    row.ins_id,
    row.ins_name,
    row.qp_id,
    row.qp_name,
    row.measure,
    row.measure_id,
    row.pt_phone,
    row.source,
    row.dos,
  ]
    .join(' ')
    .toLowerCase()

  return haystack.includes(filterValue)
}

export function filterDisplayedPatients(
  data: PatientRow[],
  sourceFilter: SourceFilterState,
  search: string,
): PatientRow[] {
  return data
    .filter((row) => matchesSourceFilter(row.source, sourceFilter))
    .filter((row) => matchesPatientSearch(row, search))
}
