import { Modal } from '@/components/ui/Modal'
import {
  APPT_FILTER_OPTIONS,
  getTodayDateString,
  toggleSourceFilterAll,
  toggleSourceFilterHedis,
  toggleSourceFilterMedAdh,
  type ApptFilterState,
  type SourceFilterState,
} from '@/types/filters'

interface FilterModalProps {
  open: boolean
  onClose: () => void
  draft: ApptFilterState
  draftSource: SourceFilterState
  onDraftChange: (next: ApptFilterState) => void
  onDraftSourceChange: (next: SourceFilterState) => void
  onApply: () => void
}

export function FilterModal({
  open,
  onClose,
  draft,
  draftSource,
  onDraftChange,
  onDraftSourceChange,
  onApply,
}: FilterModalProps) {
  const today = getTodayDateString()

  const handlePresetChange = (preset: ApptFilterState['preset']) => {
    onDraftChange({
      preset,
      customEndDate:
        preset === 'custom' ? draft.customEndDate || today : draft.customEndDate,
    })
  }

  return (
    <Modal open={open} onClose={onClose} title="Search Filters" size="md">
      <div className="filter-modal">
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
                  checked={draft.preset === option.value}
                  onChange={() => handlePresetChange(option.value)}
                />
                <span className="appt-filter__label">{option.label}</span>
              </label>
            ))}
          </div>

          {draft.preset === 'custom' && (
            <label className="filter-modal__date-field">
              <span className="filter-modal__date-label">End date</span>
              <input
                type="date"
                className="filter-modal__date-input"
                min={today}
                value={draft.customEndDate || today}
                onChange={(e) =>
                  onDraftChange({
                    ...draft,
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
