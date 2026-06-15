import { useCallback, useEffect, useId, useRef, useState } from 'react'
import clsx from 'clsx'
import { Check, ChevronDown, Loader2 } from 'lucide-react'

export interface FilterSelectOption {
  value: string
  label: string
}

interface FilterSelectProps {
  value: string
  onChange: (value: string) => void
  options: FilterSelectOption[]
  placeholder: string
  allowEmpty?: boolean
  disabled?: boolean
  loading?: boolean
  ariaLabel: string
  className?: string
}

export function FilterSelect({
  value,
  onChange,
  options,
  placeholder,
  allowEmpty = false,
  disabled = false,
  loading = false,
  ariaLabel,
  className,
}: FilterSelectProps) {
  const [open, setOpen] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)
  const listId = useId()

  const allOptions: FilterSelectOption[] = allowEmpty
    ? [{ value: '', label: placeholder }, ...options]
    : options

  const selected = options.find((option) => option.value === value)
  const displayLabel = selected?.label ?? placeholder

  const close = useCallback(() => setOpen(false), [])

  useEffect(() => {
    if (!open) return

    const handlePointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        close()
      }
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close()
    }

    document.addEventListener('mousedown', handlePointerDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('mousedown', handlePointerDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [open, close])

  const handleSelect = (nextValue: string) => {
    onChange(nextValue)
    close()
  }

  return (
    <div
      ref={rootRef}
      className={clsx('filter-select', open && 'filter-select--open', className)}
    >
      <button
        type="button"
        className="filter-select__trigger"
        onClick={() => !disabled && !loading && setOpen((prev) => !prev)}
        disabled={disabled || loading || options.length === 0}
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
      >
        <span
          className={clsx(
            'filter-select__value',
            !value && 'filter-select__value--placeholder',
          )}
        >
          {loading ? 'Loading...' : displayLabel}
        </span>
        {loading ? (
          <Loader2 size={16} className="filter-select__chevron spin" />
        ) : (
          <ChevronDown size={16} className="filter-select__chevron" />
        )}
      </button>

      {open && (
        <ul id={listId} className="filter-select__menu" role="listbox" aria-label={ariaLabel}>
          {allOptions.map((option, index) => {
            const isSelected = option.value === value
            return (
              <li key={`${option.value || '__all__'}-${index}`} role="presentation">
                <button
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  className={clsx(
                    'filter-select__option',
                    isSelected && 'filter-select__option--selected',
                  )}
                  onClick={() => handleSelect(option.value)}
                >
                  <span className="filter-select__option-label">{option.label}</span>
                  {isSelected && <Check size={16} className="filter-select__check" />}
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
