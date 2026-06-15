import { Loader2, Send } from 'lucide-react'
import { Modal } from '@/components/ui/Modal'
import type { DailyEmailPreviewSummary, SendDailyEmailRequest } from '@/api/email'

interface DailyEmailPreviewModalProps {
  open: boolean
  onClose: () => void
  payload: SendDailyEmailRequest | null
  summary: DailyEmailPreviewSummary | null
  isLoading: boolean
  isSending: boolean
  onConfirm: () => void
}

export function DailyEmailPreviewModal({
  open,
  onClose,
  payload,
  summary,
  isLoading,
  isSending,
  onConfirm,
}: DailyEmailPreviewModalProps) {
  const canSend = Boolean(payload) && !isLoading && !isSending

  return (
    <Modal open={open} onClose={onClose} title="Preview Daily Email" size="2xl">
      <div className="email-preview-modal">
        {isLoading ? (
          <div className="email-preview-modal__loading" role="status" aria-live="polite">
            <Loader2 size={28} className="spin" aria-hidden="true" />
            <p>Building today&apos;s visit report…</p>
          </div>
        ) : (
          <>
            <div className="email-preview-modal__meta">
              <div className="email-preview-modal__subject">
                <span className="email-preview-modal__label">Subject</span>
                <p className="email-preview-modal__subject-text">
                  {payload?.subject ?? '—'}
                </p>
              </div>

              {summary && (
                <div className="email-preview-modal__chips">
                  <span className="email-preview-modal__chip">
                    <strong>{summary.total}</strong> visit{summary.total === 1 ? '' : 's'}
                  </span>
                  <span className="email-preview-modal__chip email-preview-modal__chip--hedis">
                    {summary.hedisCount} HEDIS
                  </span>
                  <span className="email-preview-modal__chip email-preview-modal__chip--med">
                    {summary.medAdhCount} Med Adh
                  </span>
                  <span className="email-preview-modal__chip email-preview-modal__chip--muted">
                    {summary.reportDate}
                  </span>
                </div>
              )}
            </div>

            <div className="email-preview-modal__frame-wrap">
              {payload?.html ? (
                <iframe
                  className="email-preview-modal__frame"
                  title="Daily visit email preview"
                  srcDoc={payload.html}
                  sandbox=""
                />
              ) : (
                <div className="email-preview-modal__empty">No preview available.</div>
              )}
            </div>

            <p className="email-preview-modal__hint">
              Review the email below. When you confirm, the report is sent to the backend for
              delivery.
            </p>
          </>
        )}

        <div className="email-preview-modal__actions">
          <button
            type="button"
            className="btn btn--ghost"
            onClick={onClose}
            disabled={isSending}
          >
            Cancel
          </button>
          <button
            type="button"
            className="btn btn--primary"
            onClick={onConfirm}
            disabled={!canSend}
          >
            {isSending ? (
              <>
                <Loader2 size={16} className="spin" aria-hidden="true" />
                Sending…
              </>
            ) : (
              <>
                <Send size={16} aria-hidden="true" />
                Confirm &amp; Send
              </>
            )}
          </button>
        </div>
      </div>
    </Modal>
  )
}
