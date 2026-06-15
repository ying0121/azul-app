import * as XLSX from 'xlsx'
import { formatUsDate } from '@/lib/formatDate'
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
  'DOS',
]

function sourceLabel(source: PatientRow['source']): string {
  return source === 'hedis' ? 'HEDIS' : 'Med Adh'
}

function rowToCells(row: PatientRow): string[] {
  return [
    sourceLabel(row.source),
    row.ins_name || row.ins_id || '',
    row.qp_name || row.qp_id || '',
    `${row.pt_fname} ${row.pt_lname}`.trim(),
    row.pt_subno || '',
    row.pt_phone || '',
    formatUsDate(row.pt_dob) || '',
    row.measure || '',
    formatUsDate(row.dos) || '',
  ]
}

export function exportPatientsToExcel(rows: PatientRow[], filename: string): void {
  const worksheet = XLSX.utils.aoa_to_sheet([HEADERS, ...rows.map(rowToCells)])
  const workbook = XLSX.utils.book_new()
  XLSX.utils.book_append_sheet(workbook, worksheet, 'Patients')
  XLSX.writeFile(workbook, filename)
}
