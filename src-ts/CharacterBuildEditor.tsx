import { useEffect, useMemo, useState } from 'react'
import skillCatalogJson from './data/classic-skills-v96.json'
import { characterBuildData, editCharacterBuild } from './lib/tauri'
import type { CharacterBuildData, CharacterData, CharacterListing } from './lib/tauri'

type SkillDefinition = {
  id: number
  name: string
  tree: number
  row: number
  column: number
  requiredLevel: number
  maxLevel: number
  prerequisites: number[]
}

type ClassSkillData = { name: string; skillOffset: number; skills: SkillDefinition[] }
const skillCatalog = skillCatalogJson as Record<string, ClassSkillData>
const attributeBase: Record<string, [number, number, number, number]> = {
  Amazon: [20, 25, 20, 15], Sorceress: [10, 25, 10, 35], Necromancer: [15, 25, 15, 25],
  Paladin: [25, 20, 25, 15], Barbarian: [30, 20, 25, 10], Druid: [15, 20, 25, 20], Assassin: [20, 20, 20, 25],
}
const treeNames: Record<string, string[]> = {
  ama: ['Bow and Crossbow', 'Passive and Magic', 'Javelin and Spear'],
  sor: ['Fire', 'Lightning', 'Cold'],
  nec: ['Curses', 'Poison and Bone', 'Summoning'],
  pal: ['Combat', 'Offensive Auras', 'Defensive Auras'],
  bar: ['Combat Skills', 'Combat Masteries', 'Warcries'],
  dru: ['Summoning', 'Shape Shifting', 'Elemental'],
  ass: ['Traps', 'Shadow Disciplines', 'Martial Arts'],
}
const attributeNames = ['Strength', 'Dexterity', 'Vitality', 'Energy'] as const

interface Props {
  selected: CharacterListing
  character: CharacterData
  onSaved: (path: string) => Promise<void>
}

export default function CharacterBuildEditor({ selected, character, onSaved }: Props) {
  const [build, setBuild] = useState<CharacterBuildData | null>(null)
  const [attributes, setAttributes] = useState<[number, number, number, number]>([
    character.strength, character.dexterity, character.vitality, character.energy,
  ])
  const [skills, setSkills] = useState<number[]>([])
  const [activeTree, setActiveTree] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    void characterBuildData(selected.path).then((data) => {
      if (active) {
        setBuild(data)
        setSkills([...data.skill_levels])
      }
    }).catch((reason: unknown) => {
      if (active) setError(reason instanceof Error ? reason.message : String(reason))
    })
    return () => { active = false }
  }, [selected.path])

  const rules = build ? skillCatalog[build.class_code] : null
  const bases = attributeBase[character.class] ?? null
  const statBudget = useMemo(() => bases ? character.stat_points + attributesFromCharacter(character, bases) : 0, [bases, character])
  const skillBudget = useMemo(() => build ? character.skill_points + build.skill_levels.reduce((sum, level) => sum + level, 0) : 0, [build, character.skill_points])
  const unspentStats = bases ? statBudget - attributes.reduce((sum, value, index) => sum + value - bases[index], 0) : 0
  const unspentSkills = skillBudget - skills.reduce((sum, value) => sum + value, 0)
  const dirty = Boolean(build && (skills.some((value, index) => value !== build.skill_levels[index]) || (bases && attributes.some((value, index) => value !== [character.strength, character.dexterity, character.vitality, character.energy][index]))))
  const hasAttributeAllocations = Boolean(bases && attributes.some((value, index) => value > bases[index]))
  const hasSkillAllocations = skills.some((value) => value > 0)
  const hasAllocations = hasAttributeAllocations || hasSkillAllocations
  const activeTreeNames = build ? treeNames[build.class_code] ?? treeNames.ama : treeNames.ama

  const skillErrors = useMemo(() => {
    if (!rules || !build) return []
    const issues: string[] = []
    for (const [index, skill] of rules.skills.entries()) {
      const points = skills[index] ?? 0
      if (points > skill.maxLevel) issues.push(`${skill.name} exceeds its ${skill.maxLevel}-point limit.`)
      if (points > 0 && character.level < skill.requiredLevel) issues.push(`${skill.name} requires character level ${skill.requiredLevel}.`)
      if (points > 0 && skill.prerequisites.some((required) => !skills[required])) {
        issues.push(`${skill.name} is missing a prerequisite skill.`)
      }
    }
    return issues
  }, [build, character.level, rules, skills])

  const changeAttribute = (index: number, delta: number) => {
    if (!bases) return
    setAttributes((current) => {
      const next = [...current] as [number, number, number, number]
      const available = statBudget - current.reduce((sum, value, i) => sum + value - bases[i], 0)
      if (delta > 0 && available < delta) return current
      if (next[index] + delta < bases[index]) return current
      next[index] += delta
      return next
    })
  }

  const changeSkill = (index: number, delta: number) => {
    if (!rules) return
    setSkills((current) => {
      const skill = rules.skills[index]
      const value = current[index] ?? 0
      if (delta > 0) {
        const spent = current.reduce((sum, points) => sum + points, 0)
        const hasPrerequisites = skill.prerequisites.every((required) => current[required] > 0)
        if (character.level < skill.requiredLevel || !hasPrerequisites || value >= skill.maxLevel || skillBudget - spent < 1) return current
      } else {
        const hasAllocatedDependents = rules.skills.some((dependent, dependentIndex) =>
          (current[dependentIndex] ?? 0) > 0 && dependent.prerequisites.includes(index))
        if (value === 0 || hasAllocatedDependents) return current
      }
      const next = [...current]
      next[index] = value + delta
      return next
    })
  }

  const resetAllocations = () => {
    if (!bases || !rules) return
    setAttributes([...bases])
    setSkills(rules.skills.map(() => 0))
  }

  const resetAttributes = () => {
    if (bases) setAttributes([...bases])
  }

  const resetSkills = () => {
    if (rules) setSkills(rules.skills.map(() => 0))
  }

  const save = async () => {
    if (!build || !rules || !bases || !dirty || skillErrors.length || unspentStats < 0 || unspentSkills < 0) return
    const description = `Save these changes directly to ${selected.path}?\n\nAttributes: ${attributeNames.map((name, i) => `${name} ${[character.strength, character.dexterity, character.vitality, character.energy][i]} → ${attributes[i]}`).join(', ')}\nUnspent stat points: ${character.stat_points} → ${unspentStats}\nSkill points spent: ${build.skill_levels.reduce((sum, value) => sum + value, 0)} → ${skills.reduce((sum, value) => sum + value, 0)}\nUnspent skill points: ${character.skill_points} → ${unspentSkills}\n\nNo points are added. A verified versioned backup is created first.`
    if (!window.confirm(description)) return
    setBusy(true)
    setError(null)
    setNotice(null)
    try {
      const path = await editCharacterBuild(selected.path, attributes, skills)
      setNotice(`Character points and skills saved to ${path}.`)
      await onSaved(path)
      const updatedBuild = await characterBuildData(path)
      setBuild(updatedBuild)
      setSkills([...updatedBuild.skill_levels])
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    } finally {
      setBusy(false)
    }
  }

  if (character.save_version !== 96 || !character.stats_validated) {
    return <section className="rounded-lg border border-neutral-800 bg-neutral-900 p-4"><h2 className="text-sm font-semibold uppercase tracking-wider text-amber-200">Skills & character points</h2><p className="mt-2 text-sm text-gray-400">Editing requires a validated classic LoD version 96 character save.</p></section>
  }
  if (!build || !rules || !bases) {
    return <section className="rounded-lg border border-neutral-800 bg-neutral-900 p-4"><h2 className="text-sm font-semibold uppercase tracking-wider text-amber-200">Skills & character points</h2><p className="mt-2 text-sm text-gray-400">{error ?? 'Loading skill tree…'}</p></section>
  }

  return (
    <section className="md:col-span-2 rounded-lg border border-amber-900/60 bg-neutral-900 p-4">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wider text-amber-200">Skills & character points</h2>
          <p className="mt-1 text-xs text-gray-400">Points already spent can be reclaimed and reassigned. The editor never increases the earned point total.</p>
        </div>
        <button type="button" onClick={resetAllocations} disabled={busy || !hasAllocations} className="rounded border border-neutral-700 px-3 py-1.5 text-xs text-gray-200 hover:border-red-700 disabled:opacity-40">Respec all</button>
      </div>

      <div className="mb-5 grid gap-3 sm:grid-cols-2">
        <div className="rounded border border-neutral-800 bg-neutral-950 p-3">
          <div className="mb-2 flex items-center justify-between gap-2"><h3 className="font-semibold text-gray-200">Attributes</h3><div className="flex items-center gap-2"><span className="text-xs text-amber-300">{unspentStats} unspent</span><button type="button" onClick={resetAttributes} disabled={busy || !hasAttributeAllocations} className="text-xs text-gray-400 hover:text-white disabled:opacity-30">Reset</button></div></div>
          <div className="grid grid-cols-2 gap-2">
            {attributeNames.map((name, index) => <div key={name} className="flex items-center justify-between rounded bg-neutral-900 px-2 py-1.5 text-sm">
              <span className="text-gray-300">{name}</span><div className="flex items-center gap-2"><button type="button" aria-label={`Remove ${name} point`} disabled={busy || attributes[index] <= bases[index]} onClick={() => changeAttribute(index, -1)} className="rounded px-1 text-gray-300 hover:text-white disabled:opacity-30">−</button><span className="min-w-7 text-center font-semibold text-white">{attributes[index]}</span><button type="button" aria-label={`Add ${name} point`} disabled={busy || unspentStats < 1} onClick={() => changeAttribute(index, 1)} className="rounded px-1 text-amber-300 hover:text-amber-100 disabled:opacity-30">+</button></div>
            </div>)}
          </div>
          <p className="mt-2 text-[11px] text-gray-500">Class starting values: {bases.join(' / ')} · Changes reclaim or spend only this character’s earned stat points.</p>
        </div>
        <div className="rounded border border-neutral-800 bg-neutral-950 p-3">
          <div className="mb-2 flex items-center justify-between gap-2"><h3 className="font-semibold text-gray-200">Skill points</h3><div className="flex items-center gap-2"><span className="text-xs text-amber-300">{unspentSkills} unspent</span><button type="button" onClick={resetSkills} disabled={busy || !hasSkillAllocations} className="text-xs text-gray-400 hover:text-white disabled:opacity-30">Reset skills</button></div></div>
          <p className="text-sm text-gray-300">{skills.reduce((sum, value) => sum + value, 0)} allocated · {skillBudget} total earned</p>
          <p className="mt-2 text-[11px] text-gray-500">Skill levels include hard points only. Item bonuses are not changed.</p>
        </div>
      </div>

      <div className="mb-3 flex gap-2 border-b border-neutral-800 pb-2">
        {activeTreeNames.map((name, index) => <button key={name} type="button" onClick={() => setActiveTree(index)} className={`rounded px-3 py-1.5 text-sm ${activeTree === index ? 'bg-red-900/60 text-amber-100' : 'text-gray-400 hover:text-white'}`}>{name}</button>)}
      </div>
      <div className="grid grid-cols-3 gap-2" style={{ gridTemplateColumns: 'repeat(3, minmax(0, 1fr))', gridAutoRows: 'minmax(5.5rem, auto)' }}>
        {rules.skills.map((skill, index) => skill.tree === activeTree && <article key={skill.id} style={{ gridColumn: skill.column + 1, gridRow: skill.row + 1 }} className="flex min-w-0 flex-col justify-between rounded border border-neutral-800 bg-neutral-950 p-2">
          <div className="flex items-start justify-between gap-2"><div><h4 className="text-sm font-medium text-gray-100">{skill.name}</h4><p className="mt-0.5 text-[11px] text-gray-500">Level {skill.requiredLevel}{skill.prerequisites.length ? ` · needs ${skill.prerequisites.map((prerequisite) => rules.skills[prerequisite]?.name).join(', ')}` : ''}</p></div><span className="rounded bg-neutral-800 px-2 py-0.5 text-sm font-semibold text-amber-200">{skills[index] ?? 0}</span></div>
          <div className="mt-2 flex justify-end gap-2"><button type="button" aria-label={`Remove point from ${skill.name}`} disabled={busy || (skills[index] ?? 0) === 0 || rules.skills.some((dependent, depIndex) => (skills[depIndex] ?? 0) > 0 && dependent.prerequisites.includes(index))} onClick={() => changeSkill(index, -1)} className="h-7 w-8 rounded border border-neutral-700 text-gray-300 hover:border-red-700 disabled:opacity-30">−</button><button type="button" aria-label={`Add point to ${skill.name}`} title={character.level < skill.requiredLevel ? `Requires character level ${skill.requiredLevel}` : undefined} disabled={busy || unspentSkills < 1 || (skills[index] ?? 0) >= skill.maxLevel || character.level < skill.requiredLevel || !skill.prerequisites.every((required) => skills[required] > 0)} onClick={() => changeSkill(index, 1)} className="h-7 w-8 rounded border border-amber-900 text-amber-300 hover:bg-amber-950/50 disabled:opacity-30">+</button></div>
        </article>)}
      </div>

      {(error || skillErrors.length > 0) && <div className="mt-3 rounded border border-red-900/60 bg-red-950/30 p-2 text-xs text-red-200">{error ?? skillErrors[0]}</div>}
      {notice && <p className="mt-3 text-xs text-emerald-300">{notice}</p>}
      <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-neutral-800 pt-4"><span className="text-xs text-gray-500">A versioned backup is saved before applying changes.</span><button type="button" onClick={() => void save()} disabled={busy || !dirty || unspentStats < 0 || unspentSkills < 0 || skillErrors.length > 0} className="rounded bg-red-800 px-4 py-2 text-sm font-semibold text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-40">{busy ? 'Saving…' : 'Review and save'}</button></div>
    </section>
  )
}

function attributesFromCharacter(character: CharacterData, bases: [number, number, number, number]) {
  return [character.strength, character.dexterity, character.vitality, character.energy].reduce((sum, value, index) => sum + value - bases[index], 0)
}
