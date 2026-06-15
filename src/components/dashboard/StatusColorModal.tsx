import { useMemo } from 'react'
import { Modal } from '@/components/ui/Modal'
import { useStatusColorStore } from '@/stores/statusColorStore'
import { STATUS_COLOR_TABS, type StatusColorItem } from '@/types/statusColor'

interface StatusColorModalProps {
  open: boolean
  onClose: () => void
}

function StatusLegendCard({ item }: { item: StatusColorItem }) {
  return (
    <article
      className="status-legend-card"
      style={{ backgroundColor: item.bg_color || 'var(--surface)' }}
    >
      <h4
        className="status-legend-card__title"
        style={{ color: item.text_color || 'var(--text)' }}
      >
        {item.display}
      </h4>
      <p
        className="status-legend-card__desc"
        style={{ color: item.text_color || 'var(--text-muted)' }}
      >
        {item.description}
      </p>
    </article>
  )
}

export function StatusColorModal({ open, onClose }: StatusColorModalProps) {
  const statusColorsByType = useStatusColorStore((state) => state.statusColorsByType)
  const isLoading = useStatusColorStore((state) => state.isLoading)

  const allItems = useMemo(
    () => STATUS_COLOR_TABS.flatMap((tab) => statusColorsByType[tab.id]),
    [statusColorsByType],
  )

  return (
    <Modal open={open} onClose={onClose} title="Status Color Legend" size="lg">
      <div className="status-color-modal">
        {isLoading ? (
          <div className="status-color-modal__loading">
            <div className="spinner" />
            <p>Loading status colors...</p>
          </div>
        ) : allItems.length === 0 ? (
          <p className="status-color-modal__empty">No status colors available.</p>
        ) : (
          <div className="status-legend-list">
            {allItems.map((item) => (
              <StatusLegendCard key={`${item.type}-${item.id}`} item={item} />
            ))}
          </div>
        )}
      </div>
    </Modal>
  )
}
