export type PatientSource = 'hedis' | 'med_adh'

export interface HedisDetails {
  value1: string
  value2: string
  appt_date: string
  appt_pcp: string
  v_status: string
  v_specialty: string
  m_status: string
  p_status: string
  e_status: string
  r_status: string
  admit_date: string
  event_date: string
  discharge_date: string
  loinc1: string
  loinc2: string
  cpt1: string
  cpt2: string
  num_flag: string
  den_flag: string
}

export interface MedAdhDetails {
  med_1: string
  med_2: string
  med_3: string
  days_adherent: string | number
  num_hit: string | number
  pdc_compliant: string
  ability_compliant: string
  total_days: string | number
  min_days_80_pct: string | number
  med1_pham: string
  med1_pham_phone: string
  med1_pham_delivers: string
  med1_dos: string
  med1_days_supply: string | number
  med1_drug_quantity: string | number
  med1_refills_remain: string | number
  med1_refill_date: string
  ndc_1: string
  risk_level: string
  appt_date: string
  appt_visit: string
  event_date: string
  v_status: string
  v_specialty: string
  m_status: string
  p_status: string
  e_status: string
  r_status: string
  rx_name: string
  rx_cui: string
  rx_med_statue: string
  rx_dose_unit: string
  rx_dose_value: string
  rx_route: string
  rx_frequency: string
  rx_dispense: string
  rx_dates_given: string
  rx_refills: string
  rx_dosage: string
  num_flag: string
  den_flag: string
}

export interface PatientRowBase {
  source: PatientSource
  cyear: string | number
  ins_id: string
  ins_name: string
  qp_id: string
  qp_name: string
  measure_id: string
  obs_id: string
  measure: string
  pt_fname: string
  pt_lname: string
  pt_dob: string
  pt_gender: string
  pt_emr_id: string
  pt_subno: string
  pt_lang: string
  pt_phone: string
  pt_other_phone: string
  pt_email: string
  dos: string
}

export type HedisPatientRow = PatientRowBase & {
  source: 'hedis'
  details: HedisDetails
}

export type MedAdhPatientRow = PatientRowBase & {
  source: 'med_adh'
  details: MedAdhDetails
}

export type PatientRow = HedisPatientRow | MedAdhPatientRow

export function getPatientRowId(row: PatientRow): string {
  const segments = [
    row.source,
    row.obs_id,
    row.measure_id,
    row.pt_emr_id,
    row.pt_subno,
    row.dos,
    row.qp_id,
    row.ins_id,
    row.pt_fname,
    row.pt_lname,
  ]
    .map((value) => String(value ?? '').trim())
    .filter(Boolean)

  return segments.join('::') || 'patient-row'
}

export function isHedisRow(row: PatientRow): row is HedisPatientRow {
  return row.source === 'hedis'
}
