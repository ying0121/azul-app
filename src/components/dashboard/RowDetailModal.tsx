import { Modal } from '@/components/ui/Modal'
import { formatDetailValue, formatUsDate } from '@/lib/formatDate'
import { formatPhoneDisplay, formatPhoneDetailValue, formatPhoneTelHref, PHONE_DETAIL_KEYS } from '@/lib/formatPhone'
import { useStatusColorStore } from '@/stores/statusColorStore'
import type { HedisDetails, MedAdhDetails, PatientRow } from '@/types/patient'
import { isHedisRow } from '@/types/patient'
import { HEDIS_STATUS_DETAIL_KEYS, lookupHedisStatus } from '@/types/statusColor'
import {
  ClipboardList,
  Mail,
  Phone,
  Pill,
  Stethoscope,
  User,
} from 'lucide-react'

interface RowDetailModalProps {
  row: PatientRow | null
  onClose: () => void
}

const STATUS_KEYS = ['v_status', 'm_status', 'p_status', 'e_status', 'r_status'] as const

const hedisLabels: Record<string, string> = {
  value1: 'Value 1',
  value2: 'Value 2',
  appt_date: 'Appointment Date',
  appt_pcp: 'Appointment PCP',
  v_status: 'Visit Status',
  v_specialty: 'Visit Specialty',
  m_status: 'Measure Status',
  p_status: 'Patient Status',
  e_status: 'Event Status',
  r_status: 'Result Status',
  admit_date: 'Admit Date',
  event_date: 'Event Date',
  discharge_date: 'Discharge Date',
  loinc1: 'LOINC 1',
  loinc2: 'LOINC 2',
  cpt1: 'CPT 1',
  cpt2: 'CPT 2',
}

const medAdhLabels: Record<string, string> = {
  med_1: 'Medication 1',
  med_2: 'Medication 2',
  med_3: 'Medication 3',
  days_adherent: 'Days Adherent',
  num_hit: 'Num Hit',
  pdc_compliant: 'PDC Compliant',
  ability_compliant: 'Ability Compliant',
  total_days: 'Total Days',
  min_days_80_pct: 'Min Days 80%',
  med1_pham: 'Pharmacy',
  med1_pham_phone: 'Pharmacy Phone',
  med1_pham_delivers: 'Pharmacy Delivers',
  med1_dos: 'Med1 DOS',
  med1_days_supply: 'Days Supply',
  med1_drug_quantity: 'Drug Quantity',
  med1_refills_remain: 'Refills Remain',
  med1_refill_date: 'Refill Date',
  refill_due: 'Refill Due',
  coverage_ends: 'Coverage Ends',
  ndc_1: 'NDC 1',
  risk_level: 'Risk Level',
  appt_date: 'Appointment Date',
  appt_visit: 'Appointment Visit',
  event_date: 'Event Date',
  v_status: 'Visit Status',
  v_specialty: 'Visit Specialty',
  m_status: 'Measure Status',
  p_status: 'Patient Status',
  e_status: 'Event Status',
  r_status: 'Result Status',
  rx_name: 'Rx Name',
  rx_cui: 'Rx CUI',
  rx_med_statue: 'Rx Med Status',
  rx_dose_unit: 'Rx Dose Unit',
  rx_dose_value: 'Rx Dose Value',
  rx_route: 'Rx Route',
  rx_frequency: 'Rx Frequency',
  rx_dispense: 'Rx Dispense',
  rx_dates_given: 'Rx Dates Given',
  rx_refills: 'Rx Refills',
  rx_dosage: 'Rx Dosage',
}

const HEDIS_DETAIL_GROUPS: { title: string; keys: string[] }[] = [
  { title: 'Clinical Values', keys: ['value1', 'value2', 'loinc1', 'loinc2', 'cpt1', 'cpt2'] },
  {
    title: 'Appointments & Visits',
    keys: ['appt_date', 'appt_pcp', 'v_specialty'],
  },
  { title: 'Key Dates', keys: ['admit_date', 'event_date', 'discharge_date'] },
]

const MED_ADH_DETAIL_GROUPS: { title: string; keys: string[] }[] = [
  { title: 'Medications', keys: ['med_1', 'med_2', 'med_3', 'ndc_1', 'risk_level'] },
  {
    title: 'Adherence',
    keys: [
      'days_adherent',
      'num_hit',
      'pdc_compliant',
      'ability_compliant',
      'total_days',
      'min_days_80_pct',
    ],
  },
  {
    title: 'Pharmacy',
    keys: [
      'med1_pham',
      'med1_pham_phone',
      'med1_pham_delivers',
      'med1_dos',
      'med1_days_supply',
      'med1_drug_quantity',
      'med1_refills_remain',
      'med1_refill_date',
      'refill_due',
      'coverage_ends',
    ],
  },
  {
    title: 'Prescription',
    keys: [
      'rx_name',
      'rx_cui',
      'rx_med_statue',
      'rx_dose_unit',
      'rx_dose_value',
      'rx_route',
      'rx_frequency',
      'rx_dispense',
      'rx_dates_given',
      'rx_refills',
      'rx_dosage',
    ],
  },
  { title: 'Appointments', keys: ['appt_date', 'appt_visit', 'event_date', 'v_specialty'] },
]

function hasDetailValue(value: unknown): boolean {
  return value != null && String(value).trim() !== ''
}

function getInitials(first: string, last: string): string {
  return `${first.charAt(0)}${last.charAt(0)}`.toUpperCase() || '?'
}

function DetailField({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="detail-field">
      <dt className="detail-field__label">{label}</dt>
      <dd className="detail-field__value">{value || '—'}</dd>
    </div>
  )
}

function StatusDetailField({
  label,
  value,
  compact = false,
}: {
  label: string
  value: string | number | null | undefined
  compact?: boolean
}) {
  const hedisStatus = useStatusColorStore((s) => s.hedisStatus)
  const entry = lookupHedisStatus(hedisStatus, value)
  const rawValue = value == null || value === '' ? '' : String(value)

  if (!rawValue) {
    return compact ? (
      <span className="detail-status-chip detail-status-chip--empty">—</span>
    ) : (
      <DetailField label={label} value="—" />
    )
  }

  if (!entry) {
    return compact ? (
      <span className="detail-status-chip">{rawValue}</span>
    ) : (
      <DetailField label={label} value={rawValue} />
    )
  }

  const displayText = entry.display || entry.status || rawValue

  if (compact) {
    return (
      <span
        className="detail-status-chip"
        style={{
          color: entry.text_color || undefined,
          backgroundColor: entry.bg_color || undefined,
        }}
        title={entry.status || label}
      >
        {displayText}
      </span>
    )
  }

  return (
    <div className="detail-field">
      <dt className="detail-field__label">{label}</dt>
      <dd className="detail-field__value">
        <span
          className="status-color-badge"
          style={{
            color: entry.text_color || undefined,
            backgroundColor: entry.bg_color || undefined,
          }}
          title={entry.status || undefined}
        >
          {displayText}
        </span>
      </dd>
    </div>
  )
}

function DetailGroup({
  title,
  keys,
  details,
  labels,
}: {
  title: string
  keys: string[]
  details: HedisDetails | MedAdhDetails
  labels: Record<string, string>
}) {
  const detailRecord = details as unknown as Record<string, string | number | undefined>
  const visibleKeys = keys.filter((key) => hasDetailValue(detailRecord[key]))
  if (visibleKeys.length === 0) return null

  return (
    <div className="detail-group">
      <h4 className="detail-group__title">{title}</h4>
      <dl className="detail-grid">
        {visibleKeys.map((key) =>
          HEDIS_STATUS_DETAIL_KEYS.has(key) ? (
            <StatusDetailField
              key={key}
              label={labels[key] ?? key}
              value={detailRecord[key]}
            />
          ) : PHONE_DETAIL_KEYS.has(key) ? (
            <DetailField
              key={key}
              label={labels[key] ?? key}
              value={formatPhoneDetailValue(detailRecord[key])}
            />
          ) : (
            <DetailField
              key={key}
              label={labels[key] ?? key}
              value={formatDetailValue(key, detailRecord[key])}
            />
          ),
        )}
      </dl>
    </div>
  )
}

export function RowDetailModal({ row, onClose }: RowDetailModalProps) {
  if (!row) return null

  const isHedis = isHedisRow(row)
  const labels = isHedis ? hedisLabels : medAdhLabels
  const typeLabel = isHedis ? 'HEDIS' : 'Med Adherence'
  const detailGroups = isHedis ? HEDIS_DETAIL_GROUPS : MED_ADH_DETAIL_GROUPS
  const patientName = `${row.pt_fname} ${row.pt_lname}`

  return (
    <Modal open={!!row} onClose={onClose} title={patientName} size="xl">
      <div className="detail-modal">
        <header className={`detail-hero detail-hero--${row.source}`}>
          <div className="detail-hero__identity">
            <div className="detail-hero__avatar" aria-hidden>
              {getInitials(row.pt_fname, row.pt_lname)}
            </div>
            <div className="detail-hero__text">
              <div className="detail-hero__name-row">
                <h3 className="detail-hero__name">{patientName}</h3>
                <span className={`detail-badge detail-badge--${row.source}`}>
                  {isHedis ? <Stethoscope size={14} /> : <Pill size={14} />}
                  {typeLabel}
                </span>
                <span className="detail-hero__program">
                  <span className="detail-hero__program-label">Insurance</span>
                  <span className="detail-hero__program-value">
                    {row.ins_name || row.ins_id || '—'}
                  </span>
                </span>
                <span className="detail-hero__program">
                  <span className="detail-hero__program-label">Quality Program</span>
                  <span className="detail-hero__program-value">
                    {row.qp_name || row.qp_id || '—'}
                  </span>
                </span>
              </div>
              {(row.pt_phone || row.pt_other_phone || row.pt_email) && (
                <div className="detail-hero__contact">
                  {row.pt_phone && (
                    <a className="detail-contact-chip" href={formatPhoneTelHref(row.pt_phone)}>
                      <Phone size={14} />
                      {formatPhoneDisplay(row.pt_phone)}
                    </a>
                  )}
                  {row.pt_other_phone && (
                    <a className="detail-contact-chip" href={formatPhoneTelHref(row.pt_other_phone)}>
                      <Phone size={14} />
                      {formatPhoneDisplay(row.pt_other_phone)}
                    </a>
                  )}
                  {row.pt_email && (
                    <a className="detail-contact-chip" href={`mailto:${row.pt_email}`}>
                      <Mail size={14} />
                      {row.pt_email}
                    </a>
                  )}
                </div>
              )}
              <p className="detail-hero__meta">
                <span>MID {row.pt_subno || '—'}</span>
                <span className="detail-hero__dot" aria-hidden>
                  ·
                </span>
                <span>DOB {formatUsDate(row.pt_dob) || '—'}</span>
                <span className="detail-hero__dot" aria-hidden>
                  ·
                </span>
                <span>{row.pt_gender || '—'}</span>
              </p>
            </div>
          </div>

          <div className="detail-hero__measure">
            <ClipboardList size={18} className="detail-hero__measure-icon" />
            <div>
              <p className="detail-hero__measure-label">Measure</p>
              <p className="detail-hero__measure-value">{row.measure || '—'}</p>
            </div>
            <div className="detail-hero__measure-divider" aria-hidden />
            <div>
              <p className="detail-hero__measure-label">DOS</p>
              <p className="detail-hero__measure-value">{formatUsDate(row.dos) || '—'}</p>
            </div>
          </div>
        </header>

        <section className="detail-status-strip" aria-label="Status overview">
          {STATUS_KEYS.map((key) => (
            <div key={key} className="detail-status-strip__item">
              <span className="detail-status-strip__label">{labels[key]}</span>
              <StatusDetailField
                label={labels[key]}
                value={row.details[key]}
                compact
              />
            </div>
          ))}
        </section>

        <div className="detail-panels">
          <section className="detail-panel detail-panel--wide">
            <h3 className="detail-panel__title">
              <User size={16} />
              Patient & Contact
            </h3>
            <dl className="detail-grid">
              <DetailField label="Patient Name" value={patientName} />
              <DetailField label="MID" value={row.pt_subno} />
              <DetailField label="EMR ID" value={row.pt_emr_id} />
              <DetailField label="DOB" value={formatUsDate(row.pt_dob)} />
              <DetailField label="Gender" value={row.pt_gender} />
              <DetailField label="Language" value={row.pt_lang} />
            </dl>
          </section>
        </div>

        <section className="detail-panel detail-panel--wide">
          <h3 className="detail-panel__title">
            {isHedis ? <Stethoscope size={16} /> : <Pill size={16} />}
            {isHedis ? 'HEDIS Clinical Details' : 'Medication Adherence Details'}
          </h3>
          <div className="detail-groups">
            {detailGroups.map((group) => (
              <DetailGroup
                key={group.title}
                title={group.title}
                keys={group.keys}
                details={row.details}
                labels={labels}
              />
            ))}
          </div>
        </section>
      </div>
    </Modal>
  )
}
