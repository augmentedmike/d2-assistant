import type { BankItem, CharacterListing } from './lib/tauri'
import { classicItemIcon } from './itemIcons'
import skillCatalogJson from './data/classic-skills-v96.json'

export type Compatibility = 'usable' | 'unusable' | 'unknown'
export type BankTransferFilter = 'all' | 'available' | 'transferred'
export type BankCompatibilityFilter = 'all' | Compatibility

type SkillCatalogEntry = { skills: { id: number; name: string }[] }
const skillNamesById = new Map(
  Object.values(skillCatalogJson as Record<string, SkillCatalogEntry>)
    .flatMap(({ skills }) => skills.map(({ id, name }) => [id, name] as const)),
)
const itemStatLabels: Record<string, string> = {
  maxdamagepercent: 'Maximum Damage Percent',
  mindamagepercent: 'Minimum Damage Percent',
  maxdurabilitypercent: 'Maximum Durability Percent',
  maxhppercent: 'Maximum Life Percent',
  maxmanapercent: 'Maximum Mana Percent',
  maxdamage: 'Maximum Damage',
  mindamage: 'Minimum Damage',
  maxdurability: 'Maximum Durability',
  maxhp: 'Maximum Life',
  maxmana: 'Maximum Mana',
  attackertakesdamage: 'Attacker Takes Damage',
  attackertakeslightdamage: 'Attacker Takes Lightning Damage',
  damagetomana: 'Damage Taken Goes to Mana',
  timeduration: 'Time Duration',
  addclassskills: 'Class Skills',
  addexperience: 'Experience Gained',
  doubleherbduration: 'Double Herb Duration',
  lightradius: 'Light Radius',
  lightcolor: 'Light Color',
  reqpercent: 'Requirements Percent',
  levelreq: 'Level Requirement',
  levelreqpct: 'Level Requirement Percent',
  fasterattackrate: 'Increased Attack Speed',
  fastergethitrate: 'Faster Hit Recovery',
  fasterblockrate: 'Faster Block Rate',
  fastercastrate: 'Faster Cast Rate',
  fastermovevelocity: 'Faster Run/Walk',
  manaafterkill: 'Mana After Each Kill',
  healafterkill: 'Life After Each Kill',
  healafterdemonkill: 'Life After Each Demon Kill',
  goldbonus: 'Extra Gold from Monsters',
  magicbonus: 'Better Chance of Getting Magic Items',
  knockback: 'Knockback',
  nonclassskill: 'Non-Class Skill',
  restinpeace: 'Slain Monsters Rest in Peace',
  poisonlengthresist: 'Poison Length Reduced',
  ignoretargetac: 'Ignore Target Defense',
  fractionaltargetac: 'Target Defense',
  preventheal: 'Prevent Monster Heal',
  halffreezeduration: 'Half Freeze Duration',
  elemskill: 'Elemental Skill Damage',
  normaldamage: 'Normal Damage',
  stupidity: 'Hit Causes Monster to Flee',
  damagetargetac: 'Damage Target Defense',
  throwable: 'Throwable',
  kickdamage: 'Kick Damage',
  extrablood: 'Monsters Open Wounds',
  slow: 'Slows Target',
  aura: 'Aura',
  indesctructible: 'Indestructible',
  cannotbefrozen: 'Cannot Be Frozen',
  staminadrainpct: 'Slower Stamina Drain',
  magicarrow: 'Fires Magic Arrows',
  explosivearrow: 'Fires Explosive Arrows',
  reducedprices: 'Reduces Vendor Prices',
  reanimate: 'Reanimate',
  numsockets: 'Number of Sockets',
  skillondeath: 'Skill on Death',
  skillonlevelup: 'Skill on Level Up',
  skillongethit: 'Skill When Struck',
  tohitpercent: 'Attack Rating Percent',
  demondamagepercent: 'Damage to Demons Percent',
  undeaddamagepercent: 'Damage to Undead Percent',
  demontohit: 'Attack Rating Against Demons',
  undeadtohit: 'Attack Rating Against Undead',
  absorbfire: 'Fire Absorb',
  absorblight: 'Lightning Absorb',
  absorbcold: 'Cold Absorb',
  absorbmagic: 'Magic Absorb',
  absorblightpercent: 'Lightning Absorb Percent',
  absorbfirepercent: 'Fire Absorb Percent',
  absorbcoldpercent: 'Cold Absorb Percent',
  absorbmagicpercent: 'Magic Absorb Percent',
  chargedskill: 'Charged Skill',
  addskilltab: 'Skill Tab',
  finditem: 'Find Item',
  extrastack: 'Extra Stack',
  replenishmentdurability: 'Replenish Durability',
  replenishquantity: 'Replenish Quantity',
  armorpercent: 'Armor Percent',
  hpperlevel: 'Life per Level',
  manaperlevel: 'Mana per Level',
  findgoldperlevel: 'Gold Find per Level',
  findmagicperlevel: 'Magic Find per Level',
  findgemsperlevel: 'Gem Find per Level',
  regenstaminaperlevel: 'Stamina Regeneration per Level',
  damageaura: 'Aura Damage',
  firemindam: 'Fire Minimum Damage',
  firemaxdam: 'Fire Maximum Damage',
  lightmindam: 'Lightning Minimum Damage',
  lightmaxdam: 'Lightning Maximum Damage',
  magicmindam: 'Magic Minimum Damage',
  magicmaxdam: 'Magic Maximum Damage',
  coldmindam: 'Cold Minimum Damage',
  coldmaxdam: 'Cold Maximum Damage',
  poisonmindam: 'Poison Minimum Damage',
  poisonmaxdam: 'Poison Maximum Damage',
  lifedrainmindam: 'Life Drain Minimum Damage',
  lifedrainmaxdam: 'Life Drain Maximum Damage',
  manadrainmindam: 'Mana Drain Minimum Damage',
  manadrainmaxdam: 'Mana Drain Maximum Damage',
  stamdrainmindam: 'Stamina Drain Minimum Damage',
  stamdrainmaxdam: 'Stamina Drain Maximum Damage',
  stunlength: 'Stun Length',
}
const statWordTokens = [
  'indestructible', 'requirements', 'experience', 'lightning', 'resistance', 'maximum', 'minimum', 'duration',
  'attacker', 'monster', 'monsters', 'faster', 'slower', 'enhanced', 'strength', 'dexterity',
  'fractional', 'defense', 'damage', 'recovery', 'poison', 'physical', 'elemental', 'demon',
  'undead', 'stamina', 'drain', 'target', 'ignore', 'prevent', 'frozen', 'freeze',
  'double', 'herb', 'radius', 'color', 'normal', 'throwable', 'stupidity', 'reanimate',
  'socket', 'sockets', 'skill', 'skills', 'single', 'class', 'nonclass', 'attack',
  'block', 'rate', 'cast', 'move', 'velocity', 'run', 'walk', 'mana',
  'life', 'heal', 'after', 'each', 'kill', 'gold', 'magic', 'bonus',
  'chance', 'getting', 'items', 'rest', 'in', 'peace', 'length', 'resist',
  'half', 'light', 'fire', 'cold', 'percent', 'level', 'requirement', 'req',
  'pct', 'max', 'min', 'armor', 'add', 'reduced', 'prices', 'vendor',
  'to', 'from', 'open', 'wounds', 'blood', 'slow', 'aura', 'number',
  'thrust', 'kick', 'charged', 'per', 'time', 'by', 'regen', 'durability',
  'quantity', 'find', 'gems', 'energy', 'vitality', 'hit', 'on', 'death',
  'struck', 'causes', 'flee', 'slain', 'better', 'accuracy', 'element', 'absorb',
  'pierce', 'hp', 'num', 'cannot', 'be', 'ac', 'up', 'bone',
  'points', 'new', 'iron', 'maiden', 'last', 'frame', 'player', 'count',
  'weapon', 'chilling', 'concentration', 'conviction', 'decrepify', 'hand', 'athena', 'do',
  'overlay', 'burning', 'progressive', 'conversion', 'modifierlist', 'secondary', 'reduction', 'unit',
  'alignment', 'knockback', 'howl', 'get',
]

function splitStatIdentifier(identifier: string): string | undefined {
  const words: string[] = []
  let rest = identifier.toLowerCase().replace(/[^a-z]/g, '')
  while (rest) {
    const word = statWordTokens.filter((token) => rest.startsWith(token)).sort((a, b) => b.length - a.length)[0]
    if (!word) return undefined
    words.push(word)
    rest = rest.slice(word.length)
  }
  return words.map((word) => word[0].toUpperCase() + word.slice(1)).join(' ')
}

export type BankItemFilters = {
  search: string
  affix?: string
  type: string
  class: string
  rarity: string
  transfer: BankTransferFilter
  compatibility: BankCompatibilityFilter
  minLevel: string
  maxLevel: string
  sockets: string
}

export type BankItemDetails = {
  type: string
  rarity: string
  iconPath: string | null
  properties: string[]
  attributes: { label: string; value: string }[]
}

function parseAffixes(json: string): string[] {
  try {
    const parsed: unknown = JSON.parse(json)
    return Array.isArray(parsed) ? parsed.map(String) : []
  } catch {
    return []
  }
}

export function formatStoredProperty(value: string): string {
  const statMatch = /^(Item\s+)?(.+?)\s+([+-]?\d+)(.*)$/i.exec(value)
  if (statMatch) {
    const [, prefix = '', rawName, amount, suffix] = statMatch
    const name = rawName.toLowerCase().replace(/^item_/, '').replace(/[\s_]/g, '')
    const label = itemStatLabels[name] ?? splitStatIdentifier(name)
    if (label) value = `${prefix}${label} ${amount}${suffix}`
  }

  return value
    .replace(/\bItem\s+Single\s*Skill\s+([+-]?\d+)\s+\(layer\s+(\d+)\)/gi, (_match, amount: string, id: string) => {
      const skill = skillNamesById.get(Number(id))
      return skill ? `Item ${amount} to ${skill}` : `Item Single Skill ${amount} (skill ${id})`
    })
    .replace(/\bmanaafterkill\b/gi, 'Mana After Each Kill')
    .replace(/\bsingleskill\b/gi, 'Single Skill')
}

export function bankItemDetails(item: BankItem): BankItemDetails {
  const values = parseAffixes(item.affixes_json)
  const type = values.find((value) => value.startsWith('Type: '))?.slice(6) ?? 'Uncategorized'
  const rarity = values.find((value) => value.startsWith('Rarity: '))?.slice(8) ?? 'Unknown'
  const code = values.find((value) => value.startsWith('D2 item code: '))?.slice(14) ?? ''
  const metadataLabels = ['Type: ', 'D2 item code: ', 'Rarity: ']
  const properties = values.filter(
    (value) => !metadataLabels.some((label) => value.startsWith(label)) &&
      !['Identified', 'Unidentified', 'Ethereal', 'Non-ethereal', 'Original item record retained for transfer'].includes(value),
  ).map(formatStoredProperty)

  const attributes: BankItemDetails['attributes'] = []
  if (item.level_req < 0) {
    attributes.push({ label: 'Level req.', value: 'Unknown' })
  } else if (item.level_req > 0) {
    attributes.push({ label: 'Level req.', value: String(item.level_req) })
  }
  if (item.class_req && item.class_req !== 'Unknown') {
    attributes.push({ label: 'Class req.', value: item.class_req })
  }
  if (['Weapon', 'Armor', 'Shield'].includes(type) && item.sockets !== 0) {
    attributes.push({
      label: 'Sockets',
      value: item.sockets < 0 ? 'Socketed' : String(item.sockets),
    })
  }

  return { type, rarity, iconPath: item.icon_path ?? classicItemIcon(code), properties, attributes }
}

export function itemCompatibility(item: BankItem, character: CharacterListing | undefined): Compatibility {
  if (!character || !item.class_req || item.class_req === 'Unknown') return 'unknown'
  if (item.class_req !== 'Any' && item.class_req !== character.class) return 'unusable'
  if (item.level_req < 0) return 'unknown'
  if (character.level < item.level_req) return 'unusable'
  return 'usable'
}

function matchesSearch(item: BankItem, itemDetails: BankItemDetails, search: string): boolean {
  const terms = search.split(/\s+/).filter(Boolean)
  const name = item.base_item.toLowerCase()
  const nameWords = name.match(/[a-z0-9]+/g) ?? []
  const itemCode = parseAffixes(item.affixes_json)
    .find((value) => value.startsWith('D2 item code: '))
    ?.slice(14)
    .toLowerCase() ?? ''
  const propertyText = itemDetails.properties.join(' ').toLowerCase()
  const searchableText = [itemDetails.type, itemDetails.rarity, propertyText, item.class_req].join(' ').toLowerCase()
  const detailWords = searchableText.match(/[a-z0-9]+/g) ?? []

  return terms.every((term) => {
    if (/[^a-z0-9]/i.test(term) || /\d/.test(term)) {
      return itemCode.includes(term) || propertyText.includes(term)
    }
    return nameWords.some((word) => word.startsWith(term)) ||
      itemCode.startsWith(term) ||
      detailWords.some((word) => word.startsWith(term))
  })
}

function matchesTransfer(item: BankItem, filter: BankTransferFilter): boolean {
  if (filter === 'available') return !item.transferred_to
  if (filter === 'transferred') return Boolean(item.transferred_to)
  return true
}

function matchesCompatibility(
  item: BankItem,
  character: CharacterListing | undefined,
  filter: BankCompatibilityFilter,
): boolean {
  return filter === 'all' || itemCompatibility(item, character) === filter
}

function matchesLevelRange(item: BankItem, minimum: string, maximum: string): boolean {
  if (minimum && (item.level_req < 0 || item.level_req < Number(minimum))) return false
  if (maximum && (item.level_req < 0 || item.level_req > Number(maximum))) return false
  return true
}

export function filterBankItems(
  items: BankItem[],
  details: ReadonlyMap<number, BankItemDetails>,
  filters: BankItemFilters,
  character: CharacterListing | undefined,
): BankItem[] {
  const search = filters.search.trim().toLowerCase()
  const affix = filters.affix?.trim().toLowerCase() ?? ''
  return items.filter((item) => {
    const itemDetails = details.get(item.id) ?? bankItemDetails(item)
    if (search && !matchesSearch(item, itemDetails, search)) return false
    if (affix && !itemDetails.properties.some((property) => property.toLowerCase().includes(affix))) return false
    if (filters.type && itemDetails.type !== filters.type) return false
    if (filters.class && item.class_req !== filters.class) return false
    if (filters.rarity && itemDetails.rarity !== filters.rarity) return false
    if (!matchesTransfer(item, filters.transfer)) return false
    if (!matchesCompatibility(item, character, filters.compatibility)) return false
    if (!matchesLevelRange(item, filters.minLevel, filters.maxLevel)) return false
    if (filters.sockets && String(item.sockets) !== filters.sockets) return false
    return true
  })
}

export type GoldDraftValidation =
  | { valid: true; carriedGold: number; stashGold: number }
  | { valid: false; error: string }

export function validateGoldDrafts(
  level: number,
  carriedDraft: string,
  stashDraft: string,
): GoldDraftValidation {
  const carriedGold = Number(carriedDraft)
  const stashGold = Number(stashDraft)
  const carriedLimit = level * 10_000
  if (!Number.isInteger(carriedGold) || carriedGold < 0 || carriedGold > carriedLimit) {
    return {
      valid: false,
      error: `Carried gold must be a whole number from 0 to ${carriedLimit.toLocaleString()} at level ${level}.`,
    }
  }
  if (!Number.isInteger(stashGold) || stashGold < 0 || stashGold > 2_500_000) {
    return { valid: false, error: 'Stash gold must be a whole number from 0 to 2,500,000.' }
  }
  return { valid: true, carriedGold, stashGold }
}
