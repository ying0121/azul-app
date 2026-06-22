import { formatUsDate } from '@/lib/formatDate'
import { getRowApptDate, getRowCoverageEnds, getRowDos, getRowPcpName, getRowRefillDue, getRowValue1, getRowValue2 } from '@/lib/patientRowValues'
import type { PatientRow } from '@/types/patient'

const HEADERS = [
  'Source',
  'Insurance',
  'Quality Program',
  'Patient Name',
  'MID',
  'Phone',
  'DOB',
  'Measure',
  'Appt Date',
  'PCP Name',
  'Value 1',
  'Value 2',
  'DOS',
  'Refill Due',
  'Coverage Ends',
]

function sourceLabel(source: PatientRow['source']): string {
  return source === 'hedis' ? 'HEDIS' : 'Med Adh'
}

function rowToCells(row: PatientRow): string[] {
  const apptDate = getRowApptDate(row)
  const value1 = getRowValue1(row)
  const value2 = getRowValue2(row)
  const refillDue = getRowRefillDue(row)
  const coverageEnds = getRowCoverageEnds(row)

  return [
    sourceLabel(row.source),
    row.ins_name || row.ins_id || '',
    row.qp_name || row.qp_id || '',
    `${row.pt_fname} ${row.pt_lname}`.trim(),
    row.pt_subno || '',
    row.pt_phone || '',
    formatUsDate(row.pt_dob) || '',
    row.measure || '',
    apptDate,
    getRowPcpName(row),
    value1,
    value2,
    getRowDos(row),
    refillDue,
    coverageEnds,
  ]
}

export async function exportPatientsToExcel(rows: PatientRow[], filename: string): Promise<void> {
  const XLSX = await import('xlsx')
  const worksheet = XLSX.utils.aoa_to_sheet([HEADERS, ...rows.map(rowToCells)])
  const workbook = XLSX.utils.book_new()
  XLSX.utils.book_append_sheet(workbook, worksheet, 'Patients')
  XLSX.writeFile(workbook, filename)
}
