import { useMemo, useState } from 'react'

type Props = {
  value: string
  options: string[]
  onChange: (value: string) => void
}

export default function AffixFilter({ value, options, onChange }: Props) {
  const [open, setOpen] = useState(false)
  const { suggestions, hasMore } = useMemo(() => {
    const query = value.trim().toLowerCase()
    const matches = options.filter((option) => !query || option.toLowerCase().includes(query))
    return { suggestions: matches.slice(0, 12), hasMore: matches.length > 12 }
  }, [options, value])

  return (
    <div className="relative min-w-0">
      <div className="flex">
        <input
          type="search"
          role="combobox"
          aria-label="Filter by affix"
          aria-expanded={open}
          aria-controls="bank-affix-options"
          aria-autocomplete="list"
          autoComplete="off"
          placeholder="Filter by affix…"
          value={value}
          onFocus={() => setOpen(true)}
          onBlur={() => setOpen(false)}
          onChange={(event) => {
            onChange(event.target.value)
            setOpen(true)
          }}
          onKeyDown={(event) => {
            if (event.key === 'Escape') setOpen(false)
            if (event.key === 'Enter' && suggestions[0]) {
              event.preventDefault()
              onChange(suggestions[0])
              setOpen(false)
            }
            if (event.key === 'ArrowDown') setOpen(true)
          }}
          className="w-full min-w-0 rounded border border-neutral-700 bg-neutral-800 px-3 py-2 pr-9 text-sm text-white placeholder:text-gray-400 focus:border-red-700 focus:outline-none"
        />
        {value && (
          <button
            type="button"
            aria-label="Clear affix filter"
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => onChange('')}
            className="absolute right-2 top-1/2 -translate-y-1/2 px-1 text-gray-400 hover:text-white"
          >
            ×
          </button>
        )}
      </div>
      {open && suggestions.length > 0 && (
        <ul
          id="bank-affix-options"
          role="listbox"
          className="absolute z-30 mt-1 max-h-64 w-full overflow-y-auto rounded border border-neutral-700 bg-neutral-950 py-1 shadow-xl"
        >
          {suggestions.map((option) => (
            <li key={option} role="option" aria-selected={option === value}>
              <button
                type="button"
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  onChange(option)
                  setOpen(false)
                }}
                className="w-full px-3 py-2 text-left text-xs text-amber-100 hover:bg-neutral-800"
              >
                {option}
              </button>
            </li>
          ))}
          {hasMore && (
            <li className="px-3 py-2 text-[10px] text-gray-500" aria-hidden="true">Keep typing to narrow the add list.</li>
          )}
        </ul>
      )}
    </div>
  )
}
