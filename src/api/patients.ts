import { apiClient, USE_MOCK } from './client'
import { retryAsync } from '@/lib/retryAsync'
import { getClinicYear } from '@/lib/clinicDate'
import { saveHuddleToken } from '@/lib/session'
import { mockPatients, mockPcpList } from './mock'
import type { PatientFilters } from '@/types/filters'
import type {
  HedisDetails,
  MedAdhDetails,
  PatientRow,
  PatientSource,
} from '@/types/patient'

interface PatientsApiResponse {
  status: string
  data?: unknown[]
  token?: string
  message?: string
}

export interface PatientsFetchResult {
  rows: PatientRow[]
  token: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function readString(data: Record<string, unknown>, ...keys: string[]): string {
  for (const key of keys) {
    const value = data[key]
    if (typeof value === 'string') return value
    if (typeof value === 'number' && !Number.isNaN(value)) return String(value)
  }
  return ''
}

function readScalar(data: Record<string, unknown>, ...keys: string[]): string | number {
  for (const key of keys) {
    const value = data[key]
    if (typeof value === 'string') return value
    if (typeof value === 'number' && !Number.isNaN(value)) return value
  }
  return ''
}

function readSource(data: Record<string, unknown>): PatientSource | null {
  const raw = readString(data, 'source', 'Source').toLowerCase()
  if (raw === 'hedis') return 'hedis'
  if (raw === 'med_adh') return 'med_adh'
  return null
}

function readDetails(data: Record<string, unknown>): Record<string, unknown> {
  const nested = data.details ?? data.Details
  if (isRecord(nested)) return nested
  return {}
}

function mapDetails(
  source: PatientSource,
  details: Record<string, unknown>,
  item: Record<string, unknown>,
) {
  const read = (...keys: string[]) => readString(details, ...keys)
  const readNum = (...keys: string[]) => readScalar(details, ...keys)
  const readItem = (...keys: string[]) => readString(item, ...keys)

  if (source === 'hedis') {
    return {
      value1: read('value1', 'Value1'),
      value2: read('value2', 'Value2'),
      appt_date: read('appt_date', 'ApptDate'),
      appt_pcp: read('appt_pcp', 'ApptPcp'),
      v_status: read('v_status', 'VStatus') || readItem('v_status', 'VStatus'),
      v_specialty: read('v_specialty', 'VSpecialty', 'vspecialty'),
      m_status: read('m_status', 'MStatus') || readItem('m_status', 'MStatus'),
      p_status: read('p_status', 'PStatus') || readItem('p_status', 'PStatus'),
      e_status: read('e_status', 'EStatus') || readItem('e_status', 'EStatus'),
      r_status: read('r_status', 'RStatus') || readItem('r_status', 'RStatus'),
      admit_date: read('admit_date', 'AdmitDate'),
      event_date: read('event_date', 'EventDate'),
      discharge_date: read('discharge_date', 'DischargeDate'),
      loinc1: read('loinc1', 'Loinc1'),
      loinc2: read('loinc2', 'Loinc2'),
      cpt1: read('cpt1', 'Cpt1'),
      cpt2: read('cpt2', 'Cpt2'),
      num_flag: read('num_flag', 'NumFlag'),
      den_flag: read('den_flag', 'DenFlag'),
    } satisfies HedisDetails
  }

  return {
    med_1: read('med_1', 'Med1'),
    med_2: read('med_2', 'Med2'),
    med_3: read('med_3', 'Med3'),
    days_adherent: readNum('days_adherent', 'DaysAdherent'),
    num_hit: readNum('num_hit', 'NumHit'),
    pdc_compliant: read('pdc_compliant', 'PdcCompliant'),
    ability_compliant: read('ability_compliant', 'AbilityCompliant'),
    total_days: readNum('total_days', 'TotalDays'),
    min_days_80_pct: readNum('min_days_80_pct', 'MinDays80Pct'),
    med1_pham: read('med1_pham', 'Med1Pham', 'med1_pharmacy', 'Med1Pharmacy'),
    med1_pham_phone: read('med1_pham_phone', 'Med1PhamPhone', 'med1_pharmacy_phone'),
    med1_pham_delivers: read(
      'med1_pham_delivers',
      'Med1PhamDelivers',
      'med1_pharmacy_delivers',
    ),
    med1_dos: read('med1_dos', 'Med1Dos'),
    med1_days_supply: readNum('med1_days_supply', 'Med1DaysSupply'),
    med1_drug_quantity: readNum('med1_drug_quantity', 'Med1DrugQuantity'),
    med1_refills_remain: readNum('med1_refills_remain', 'Med1RefillsRemain'),
    med1_refill_date: read('med1_refill_date', 'Med1RefillDate'),
    refill_due: read('refill_due', 'RefillDue') || readItem('refill_due', 'RefillDue'),
    coverage_ends:
      read('coverage_ends', 'CoverageEnds', 'coverage_end', 'CoverageEnd') ||
      readItem('coverage_ends', 'CoverageEnds', 'coverage_end', 'CoverageEnd'),
    ndc_1: read('ndc_1', 'Ndc1'),
    risk_level: read('risk_level', 'RiskLevel'),
    appt_date: read('appt_date', 'ApptDate') || readItem('appt_date', 'ApptDate'),
    appt_visit: read('appt_visit', 'ApptVisit'),
    event_date: read('event_date', 'EventDate'),
    v_status: read('v_status', 'VStatus') || readItem('v_status', 'VStatus'),
    v_specialty: read('v_specialty', 'VSpecialty'),
    m_status: read('m_status', 'MStatus') || readItem('m_status', 'MStatus'),
    p_status: read('p_status', 'PStatus') || readItem('p_status', 'PStatus'),
    e_status: read('e_status', 'EStatus') || readItem('e_status', 'EStatus'),
    r_status: read('r_status', 'RStatus') || readItem('r_status', 'RStatus'),
    rx_name: read('rx_name', 'RxName'),
    rx_cui: read('rx_cui', 'RxCui'),
    rx_med_statue: read('rx_med_statue', 'RxMedStatue', 'rx_med_status'),
    rx_dose_unit: read('rx_dose_unit', 'RxDoseUnit'),
    rx_dose_value: read('rx_dose_value', 'RxDoseValue'),
    rx_route: read('rx_route', 'RxRoute'),
    rx_frequency: read('rx_frequency', 'RxFrequency'),
    rx_dispense: read('rx_dispense', 'RxDispense'),
    rx_dates_given: read('rx_dates_given', 'RxDatesGiven'),
    rx_refills: read('rx_refills', 'RxRefills'),
    rx_dosage: read('rx_dosage', 'RxDosage'),
    num_flag: read('num_flag', 'NumFlag'),
    den_flag: read('den_flag', 'DenFlag'),
  } satisfies MedAdhDetails
}

function parsePatientRow(item: unknown): PatientRow | null {
  if (!isRecord(item)) return null

  const source = readSource(item)
  if (!source) return null

  const base = {
    source,
    cyear: readScalar(item, 'cyear', 'Cyear', 'cYear'),
    ins_id: readString(item, 'ins_id', 'insId', 'InsId'),
    ins_name: readString(item, 'ins_name', 'insName', 'InsName'),
    qp_id: readString(item, 'qp_id', 'qpId', 'QpId'),
    qp_name: readString(item, 'qp_name', 'qpName', 'QpName'),
    measure_id: readString(item, 'measure_id', 'measureId', 'MeasureId'),
    obs_id: readString(item, 'obs_id', 'obsId', 'ObsId'),
    measure: readString(item, 'measure', 'Measure', 'measure_name', 'MeasureName'),
    pt_fname: readString(item, 'pt_fname', 'ptFname', 'PtFname'),
    pt_lname: readString(item, 'pt_lname', 'ptLname', 'PtLname'),
    pt_dob: readString(item, 'pt_dob', 'ptDob', 'PtDob'),
    pt_gender: readString(item, 'pt_gender', 'ptGender', 'PtGender'),
    pt_emr_id: readString(item, 'pt_emr_id', 'ptEmrId', 'PtEmrId'),
    pt_subno: readString(item, 'pt_subno', 'ptSubno', 'PtSubno'),
    pt_lang: readString(item, 'pt_lang', 'ptLang', 'PtLang'),
    pt_phone: readString(item, 'pt_phone', 'ptPhone', 'PtPhone'),
    pt_other_phone: readString(item, 'pt_other_phone', 'ptOtherPhone', 'PtOtherPhone'),
    pt_email: readString(item, 'pt_email', 'ptEmail', 'PtEmail'),
    dos: readString(item, 'dos', 'Dos', 'DOS'),
    pcp_fname: readString(item, 'pcp_fname', 'pcpFname', 'PcpFname'),
    pcp_lname: readString(item, 'pcp_lname', 'pcpLname', 'PcpLname'),
  }

  const details = mapDetails(source, readDetails(item), item)

  if (source === 'hedis') {
    return { ...base, source: 'hedis', details: details as HedisDetails }
  }

  return { ...base, source: 'med_adh', details: details as MedAdhDetails }
}

function filterMockPatients(filters: PatientFilters): PatientRow[] {
  const selectedPcpName =
    filters.pcp_id && filters.pcp_id !== '0'
      ? mockPcpList.find((option) => option.pcp_id === filters.pcp_id)?.pcp_name
          .trim()
          .toLowerCase()
      : null

  return mockPatients.filter((row) => {
    if (filters.ins_id && filters.ins_id !== '0' && row.ins_id !== filters.ins_id) return false
    if (filters.qp_id && filters.qp_id !== '0' && row.qp_id !== filters.qp_id) return false
    if (selectedPcpName) {
      const rowPcpName = `${row.pcp_fname} ${row.pcp_lname}`.trim().toLowerCase()
      if (rowPcpName !== selectedPcpName) return false
    }
    if (filters.measures) {
      const selectedMeasureIds = filters.measures.split(',').map((id) => id.trim()).filter(Boolean)
      if (selectedMeasureIds.length > 0 && !selectedMeasureIds.includes(row.measure_id)) {
        return false
      }
    }
    return true
  })
}

export async function fetchPatients(
  filters: PatientFilters = {},
): Promise<PatientsFetchResult> {
  if (USE_MOCK) {
    await new Promise((r) => setTimeout(r, 400))
    const token = 'mock-huddle-token'
    saveHuddleToken(token)
    return { rows: filterMockPatients(filters), token }
  }

  const payload: Record<string, string | number> = {
    clinic_id: filters.clinic_id ?? '',
    ins_id: filters.ins_id ?? '',
    qp_id: filters.qp_id ?? '',
    pcp_id: filters.pcp_id && filters.pcp_id !== '' ? filters.pcp_id : '0',
    cyear: filters.cyear ?? getClinicYear(),
    filter: filters.filter ?? '',
    measures: filters.measures ?? '',
    appt_start: filters.appt_start ?? '',
    appt_end: filters.appt_end ?? '',
  }

  const { data } = await retryAsync(() =>
    apiClient.post<PatientsApiResponse>('/daily-huddle/', payload),
  )

  if (!isRecord(data) || data.status !== 'success' || !Array.isArray(data.data)) {
    throw new Error(
      isRecord(data) && typeof data.message === 'string'
        ? data.message
        : 'Unable to load patient data.',
    )
  }

  const token = typeof data.token === 'string' ? data.token.trim() : ''
  if (token) {
    saveHuddleToken(token)
  }

  const rows = data.data
    .map(parsePatientRow)
    .filter((row): row is PatientRow => row != null)

  return { rows, token }
}
