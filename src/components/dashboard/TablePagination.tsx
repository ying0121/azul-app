import clsx from 'clsx'
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
} from 'lucide-react'

type PageItem = number | 'ellipsis'

function buildPageItems(pageIndex: number, pageCount: number): PageItem[] {
  if (pageCount <= 0) return []

  if (pageCount <= 7) {
    return Array.from({ length: pageCount }, (_, index) => index)
  }

  const items: PageItem[] = [0]
  const start = Math.max(1, pageIndex - 1)
  const end = Math.min(pageCount - 2, pageIndex + 1)

  if (start > 1) items.push('ellipsis')
  for (let index = start; index <= end; index += 1) {
    if (!items.includes(index)) items.push(index)
  }
  if (end < pageCount - 2) items.push('ellipsis')

  const lastPage = pageCount - 1
  if (!items.includes(lastPage)) items.push(lastPage)

  return items
}

interface TablePaginationProps {
  pageIndex: number
  pageCount: number
  pageSize: number
  totalRows: number
  onPageChange: (pageIndex: number) => void
}

export function TablePagination({
  pageIndex,
  pageCount,
  pageSize,
  totalRows,
  onPageChange,
}: TablePaginationProps) {
  const safePageCount = Math.max(pageCount, 0)
  const canPrevious = safePageCount > 0 && pageIndex > 0
  const canNext = safePageCount > 0 && pageIndex < safePageCount - 1
  const pageItems = buildPageItems(pageIndex, safePageCount)

  const rangeStart = totalRows === 0 ? 0 : pageIndex * pageSize + 1
  const rangeEnd = Math.min((pageIndex + 1) * pageSize, totalRows)

  return (
    <div className="data-table-pagination">
      <span className="data-table-pagination__summary">
        {totalRows === 0
          ? 'No records'
          : `Showing ${rangeStart}–${rangeEnd} of ${totalRows}`}
      </span>

      <div className="data-table-pagination__controls">
        <button
          type="button"
          className="data-table-pagination__nav"
          onClick={() => onPageChange(0)}
          disabled={!canPrevious}
          aria-label="First page"
          title="First page"
        >
          <ChevronsLeft size={16} />
        </button>

        <button
          type="button"
          className="data-table-pagination__nav"
          onClick={() => onPageChange(pageIndex - 1)}
          disabled={!canPrevious}
          aria-label="Previous page"
          title="Previous page"
        >
          <ChevronLeft size={16} />
          <span>Previous</span>
        </button>

        <div className="data-table-pagination__pages" role="group" aria-label="Page numbers">
          {pageItems.map((item, index) =>
            item === 'ellipsis' ? (
              <span
                key={`ellipsis-${index}`}
                className="data-table-pagination__ellipsis"
                aria-hidden
              >
                …
              </span>
            ) : (
              <button
                key={`page-${item}-${index}`}
                type="button"
                className={clsx(
                  'data-table-pagination__page',
                  item === pageIndex && 'data-table-pagination__page--active',
                )}
                onClick={() => onPageChange(item)}
                aria-label={`Page ${item + 1}`}
                aria-current={item === pageIndex ? 'page' : undefined}
              >
                {item + 1}
              </button>
            ),
          )}
        </div>

        <button
          type="button"
          className="data-table-pagination__nav"
          onClick={() => onPageChange(pageIndex + 1)}
          disabled={!canNext}
          aria-label="Next page"
          title="Next page"
        >
          <span>Next</span>
          <ChevronRight size={16} />
        </button>

        <button
          type="button"
          className="data-table-pagination__nav"
          onClick={() => onPageChange(safePageCount - 1)}
          disabled={!canNext}
          aria-label="Last page"
          title="Last page"
        >
          <ChevronsRight size={16} />
        </button>
      </div>
    </div>
  )
}
