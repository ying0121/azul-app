import { Modal } from '@/components/ui/Modal'
import { FilterSelect } from '@/components/ui/FilterSelect'
import { FilterMultiSelect } from '@/components/ui/FilterMultiSelect'
import {
  ALL_MEASURES_PLACEHOLDER,
  APPT_FILTER_OPTIONS,
  getTodayDateString,
  toggleSourceFilterAll,
  toggleSourceFilterHedis,
  toggleSourceFilterMedAdh,
  type ApptFilterState,
  type InsuranceOption,
  type MeasureOption,
  type PcpOption,
  type QualityProgramOption,
  type SourceFilterState,
} from '@/types/filters'

interface FilterModalProps {
  open: boolean
  onClose: () => void
  draftInsuranceId: string
  draftQualityProgramId: string
  draftPcpId: string
  draftMeasureIds: string[]
  draftAppt: ApptFilterState
  draftSource: SourceFilterState
  insuranceOptions: InsuranceOption[]
  qualityProgramOptions: QualityProgramOption[]
  pcpOptions: PcpOption[]
  measureOptions: MeasureOption[]
  onDraftInsuranceChange: (insId: string) => void
  onDraftQualityProgramChange: (qpId: string) => void
  onDraftPcpChange: (pcpId: string) => void
  onDraftMeasureIdsChange: (measureIds: string[]) => void
  onDraftApptChange: (next: ApptFilterState) => void
  onDraftSourceChange: (next: SourceFilterState) => void
  isInsuranceLoading?: boolean
  isQualityProgramLoading?: boolean
  isPcpLoading?: boolean
  isMeasureLoading?: boolean
  onApply: () => void
}

export function FilterModal({
  open,
  onClose,
  draftInsuranceId,
  draftQualityProgramId,
  draftPcpId,
  draftMeasureIds,
  draftAppt,
  draftSource,
  insuranceOptions,
  qualityProgramOptions,
  pcpOptions,
  measureOptions,
  onDraftInsuranceChange,
  onDraftQualityProgramChange,
  onDraftPcpChange,
  onDraftMeasureIdsChange,
  onDraftApptChange,
  onDraftSourceChange,
  isInsuranceLoading,
  isQualityProgramLoading,
  isPcpLoading,
  isMeasureLoading,
  onApply,
}: FilterModalProps) {
  const today = getTodayDateString()

  const handlePresetChange = (preset: ApptFilterState['preset']) => {
    onDraftApptChange({
      preset,
      customEndDate:
        preset === 'custom' ? draftAppt.customEndDate || today : draftAppt.customEndDate,
    })
  }

  return (
    <Modal open={open} onClose={onClose} title="Filters" size="md">
      <div className="filter-modal">
        <section className="filter-modal__section">
          <h3 className="filter-modal__heading">Insurance</h3>
          <FilterSelect
            value={draftInsuranceId}
            onChange={onDraftInsuranceChange}
            options={insuranceOptions.map((option) => ({
              value: option.ins_id,
              label: option.ins_name,
            }))}
            placeholder="Insurance"
            disabled={isInsuranceLoading}
            loading={isInsuranceLoading}
            ariaLabel="Filter by insurance"
            className="filter-modal__select"
          />
        </section>

        <section className="filter-modal__section">
          <h3 className="filter-modal__heading">Quality Program</h3>
          <FilterSelect
            value={draftQualityProgramId}
            onChange={onDraftQualityProgramChange}
            options={qualityProgramOptions.map((option) => ({
              value: option.qp_id,
              label: option.qp_name,
            }))}
            placeholder="Quality program"
            disabled={!draftInsuranceId || isQualityProgramLoading}
            loading={isQualityProgramLoading}
            ariaLabel="Filter by quality program"
            className="filter-modal__select"
          />
        </section>

        <section className="filter-modal__section">
          <h3 className="filter-modal__heading">PCP</h3>
          <FilterSelect
            value={draftPcpId}
            onChange={onDraftPcpChange}
            options={pcpOptions.map((option) => ({
              value: option.pcp_id,
              label: option.pcp_name,
            }))}
            placeholder="PCP"
            disabled={isPcpLoading}
            loading={isPcpLoading}
            ariaLabel="Filter by PCP name"
            className="filter-modal__select"
          />
        </section>

        <section className="filter-modal__section">
          <h3 className="filter-modal__heading">Measure</h3>
          <FilterMultiSelect
            values={draftMeasureIds}
            onChange={onDraftMeasureIdsChange}
            options={measureOptions.map((option) => ({
              value: option.measure_id,
              label: option.measure,
            }))}
            placeholder={ALL_MEASURES_PLACEHOLDER}
            disabled={isMeasureLoading}
            loading={isMeasureLoading}
            ariaLabel="Filter by measure"
            className="filter-modal__select"
          />
        </section>

        <section className="filter-modal__section">
          <h3 className="filter-modal__heading">Source</h3>
          <div className="source-filter" role="group" aria-label="Filter by source">
            <label className="source-filter__option">
              <input
                type="checkbox"
                className="source-filter__input"
                checked={draftSource.all}
                onChange={() => onDraftSourceChange(toggleSourceFilterAll())}
              />
              <span className="source-filter__label">All</span>
            </label>

            <label className="source-filter__option">
              <input
                type="checkbox"
                className="source-filter__input"
                checked={!draftSource.all && draftSource.hedis}
                onChange={() => onDraftSourceChange(toggleSourceFilterHedis(draftSource))}
              />
              <span className="source-filter__label">HEDIS</span>
            </label>

            <label className="source-filter__option">
              <input
                type="checkbox"
                className="source-filter__input"
                checked={!draftSource.all && draftSource.med_adh}
                onChange={() => onDraftSourceChange(toggleSourceFilterMedAdh(draftSource))}
              />
              <span className="source-filter__label">Med Adh</span>
            </label>
          </div>
        </section>

        <section className="filter-modal__section">
          <h3 className="filter-modal__heading">Appointment Date</h3>
          <div className="appt-filter" role="radiogroup" aria-label="Appointment date range">
            {APPT_FILTER_OPTIONS.map((option) => (
              <label key={option.value} className="appt-filter__option">
                <input
                  type="radio"
                  name="appt-filter"
                  className="appt-filter__input"
                  checked={draftAppt.preset === option.value}
                  onChange={() => handlePresetChange(option.value)}
                />
                <span className="appt-filter__label">{option.label}</span>
              </label>
            ))}
          </div>

          {draftAppt.preset === 'custom' && (
            <label className="filter-modal__date-field">
              <span className="filter-modal__date-label">End date</span>
              <input
                type="date"
                className="filter-modal__date-input"
                min={today}
                value={draftAppt.customEndDate || today}
                onChange={(e) =>
                  onDraftApptChange({
                    ...draftAppt,
                    customEndDate: e.target.value,
                  })
                }
              />
            </label>
          )}
        </section>

        <div className="filter-modal__actions">
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn btn--primary" onClick={onApply}>
            Apply Filters
          </button>
        </div>
      </div>
    </Modal>
  )
}
