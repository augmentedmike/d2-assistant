import { useMemo, useState } from 'react'
import catalog from './data/runewords.json'

type Recipe = {
  id: string
  name: string
  sockets: number
  bases: string[]
  runes: string[]
  level: number
  patch: string
  ladderOnly: boolean
  d2rOnly: boolean
  effects: string[]
  availability?: string[]
  sourceNote?: string
  ladder?: boolean
}

const recipes = catalog as Recipe[]
const rulesets = ['Standard LoD', 'LoD Ladder', 'D2R', 'Reign of the Warlock'] as const
type Ruleset = typeof rulesets[number]

function compatible(recipe: Recipe, ruleset: Ruleset): boolean {
  if (recipe.name === 'Hustle' && ruleset === 'Reign of the Warlock') return false
  if (recipe.patch.startsWith('RotW')) return ruleset === 'Reign of the Warlock'
  if (recipe.d2rOnly) return ruleset === 'D2R' || ruleset === 'Reign of the Warlock'
  if (recipe.ladderOnly) return ruleset !== 'Standard LoD'
  return true
}

function status(recipe: Recipe, ruleset: Ruleset): string {
  if (recipe.name === 'Hustle' && ruleset === 'Reign of the Warlock') return 'Legacy D2R name · see Mania / Hysteria'
  if (recipe.patch.startsWith('RotW')) return 'Reign of the Warlock only · expansion required'
  if (recipe.d2rOnly) return `D2R only · patch ${recipe.patch}`
  if (recipe.ladderOnly) return ruleset === 'Standard LoD' ? 'LoD Ladder only' : 'LoD Ladder · D2R available'
  return 'Standard LoD'
}

export default function Runewords({ showExtendedRulesets }: { showExtendedRulesets: boolean }) {
  const [selectedRuleset, setSelectedRuleset] = useState<Ruleset>('Standard LoD')
  const ruleset = !showExtendedRulesets && selectedRuleset !== 'Standard LoD' && selectedRuleset !== 'LoD Ladder'
    ? 'Standard LoD'
    : selectedRuleset
  const [search, setSearch] = useState('')
  const [socketCount, setSocketCount] = useState('')
  const [base, setBase] = useState('')
  const [availability, setAvailability] = useState('')

  const catalogRecipes = useMemo(() => showExtendedRulesets ? recipes : recipes.filter((recipe) => !recipe.d2rOnly), [showExtendedRulesets])
  const availableRulesets = showExtendedRulesets ? rulesets : rulesets.slice(0, 2)

  const bases = useMemo(() => [...new Set(catalogRecipes.flatMap((recipe) => recipe.bases))].sort(), [catalogRecipes])
  const visibleRecipes = catalogRecipes.filter((recipe) => {
    const term = search.trim().toLowerCase()
    const text = [recipe.name, recipe.runes.join(' '), recipe.bases.join(' '), recipe.effects.join(' ')].join(' ').toLowerCase()
    return (!term || text.includes(term)) && (!socketCount || recipe.sockets === Number(socketCount)) &&
      (!base || recipe.bases.includes(base)) && (!availability || status(recipe, ruleset).toLowerCase().includes(availability.toLowerCase()))
  }).sort((a, b) => a.name.localeCompare(b.name) || a.patch.localeCompare(b.patch))

  return <main className="flex-1 min-h-0 overflow-y-auto bg-neutral-950 p-4 sm:p-6 text-left">
    <div className="mx-auto max-w-[1500px]">
      <div className="mb-5 flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Runeword recipes</h1>
          <p className="mt-1 max-w-3xl text-sm text-gray-400">Ordered recipes, socket counts, and valid base types. The catalog works offline.</p>
        </div>
        <label className="flex flex-col gap-1 text-xs font-medium text-gray-400">Game ruleset
          <select value={ruleset} onChange={(event) => setSelectedRuleset(event.target.value as Ruleset)} className="min-w-64 rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm text-white">
            {availableRulesets.map((value) => <option key={value}>{value}</option>)}
          </select>
        </label>
      </div>
      <div className="mb-5 rounded-lg border border-amber-500/30 bg-amber-950/30 p-3 text-sm text-amber-100">
        <strong>Selected ruleset: {ruleset}.</strong> Use a nonmagical socketed base with exactly the listed sockets and insert runes in order. Recipes tagged D2R or Reign of the Warlock do not work in original LoD.
      </div>
      <div className="mb-5 grid grid-cols-1 gap-2 sm:grid-cols-2 xl:grid-cols-4">
        <input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Search name, rune, or base" className="rounded border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm text-white placeholder:text-gray-500" />
        <select value={socketCount} onChange={(event) => setSocketCount(event.target.value)} className="rounded border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm text-white"><option value="">Any socket count</option>{[2,3,4,5,6].map((n) => <option key={n} value={n}>{n} sockets</option>)}</select>
        <select value={base} onChange={(event) => setBase(event.target.value)} className="rounded border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm text-white"><option value="">Any base type</option>{bases.map((value) => <option key={value}>{value}</option>)}</select>
        <select value={availability} onChange={(event) => setAvailability(event.target.value)} className="rounded border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm text-white"><option value="">Any availability</option><option>Standard LoD</option><option>Ladder</option>{showExtendedRulesets && <><option>D2R</option><option>Reign</option></>}</select>
      </div>
      <div className="mb-3 text-xs text-gray-500">{visibleRecipes.length} recipes · names, bases, and ordered runes are shown</div>
      {visibleRecipes.length === 0 ? <div className="rounded-lg border border-dashed border-neutral-800 p-10 text-center text-sm text-gray-400">No recipes match. Try another ruleset or clear a filter.</div> :
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 2xl:grid-cols-3">
          {visibleRecipes.map((recipe) => {
            const allowed = compatible(recipe, ruleset)
            return <article key={recipe.id} className={`min-w-0 rounded-lg border bg-neutral-900/70 p-4 shadow-sm ${allowed ? 'border-neutral-800' : 'border-red-900/70'}`}>
              <header className="flex flex-wrap items-start justify-between gap-2">
                <div><h2 className="text-lg font-semibold text-white">{recipe.name}</h2><p className="mt-1 text-xs text-gray-400">{recipe.sockets} sockets · level {recipe.level} · patch {recipe.patch}</p></div>
                <span className={`rounded-full border px-2 py-1 text-[11px] ${allowed ? 'border-amber-500/30 bg-amber-500/10 text-amber-200' : 'border-red-500/30 bg-red-500/10 text-red-300'}`}>{status(recipe, ruleset)}</span>
              </header>
              {recipe.ladder && <p className="mt-2 inline-flex rounded border border-orange-500/30 bg-orange-500/10 px-2 py-1 text-[11px] text-orange-200">Ladder / Offline variant</p>}
              {recipe.sourceNote && <p className="mt-2 text-xs leading-relaxed text-orange-200/80">{recipe.sourceNote}</p>}
              {!allowed && <p className="mt-3 rounded bg-red-950/40 px-2 py-1.5 text-xs text-red-200">Not available in {ruleset}.</p>}
              <div className="mt-4"><div className="mb-1 text-[11px] uppercase tracking-wide text-gray-500">Rune order</div><div className="flex flex-wrap items-center gap-1.5">{recipe.runes.map((rune, index) => <span key={`${index}-${rune}`} className="flex items-center gap-1.5"><span className="rounded border border-amber-500/20 bg-amber-500/10 px-2 py-1 text-xs font-semibold text-amber-200">{rune}</span>{index < recipe.runes.length - 1 && <span className="text-gray-600">→</span>}</span>)}</div></div>
              <div className="mt-3"><div className="mb-1 text-[11px] uppercase tracking-wide text-gray-500">Valid bases</div><p className="text-sm text-gray-200">{recipe.bases.join(' · ')}</p></div>
              <details className="mt-3 border-t border-neutral-800 pt-2"><summary className="cursor-pointer text-xs text-gray-400 hover:text-gray-200">Show item bonuses</summary><ul className="mt-2 space-y-1">{recipe.effects.map((effect) => <li key={effect} className="text-xs leading-relaxed text-amber-100">{effect}</li>)}</ul></details>
            </article>
          })}
        </div>}
    </div>
  </main>
}
