import { describe, expect, test } from 'bun:test'
import type { BankItem, CharacterListing } from '../src-ts/lib/tauri'
import { bankItemDetails, filterBankItems, itemCompatibility, validateGoldDrafts } from '../src-ts/bankItemPresentation'

const makeItem = (updates: Partial<BankItem> = {}): BankItem => ({
  id: 1,
  base_item: 'Crystal Sword',
  affixes_json: JSON.stringify([
    'Type: Weapon',
    'D2 item code: crs',
    'Rarity: Normal',
    'Identified',
    'Ethereal',
    'Original item record retained for transfer',
    '+3 to Fire Skills',
  ]),
  level_req: 25,
  class_req: 'Any',
  sockets: 4,
  icon_path: null,
  source_character: 'Test',
  banked_at: 1,
  updated_at: 1,
  has_item_data: true,
  transferred_to: null,
  ...updates,
})

const makeCharacter = (updates: Partial<CharacterListing> = {}): CharacterListing => ({
  path: '/saves/hero.d2s',
  name: 'Hero',
  class: 'Amazon',
  level: 30,
  file_size: 100,
  modified: 1,
  backup_warning: null,
  ...updates,
})

describe('bankItemDetails', () => {
  test('separates display metadata from actual item properties', () => {
    const details = bankItemDetails(makeItem())
    expect(details.type).toBe('Weapon')
    expect(details.rarity).toBe('Normal')
    expect(details.iconPath).toBe('/item-icons/crs.png')
    expect(details.properties).toEqual(['+3 to Fire Skills'])
    expect(details.attributes).toEqual([
      { label: 'Level req.', value: '25' },
      { label: 'Class req.', value: 'Any' },
      { label: 'Sockets', value: '4' },
    ])
  })

  test('uses a supplied item icon path before the classic item mapping', () => {
    expect(bankItemDetails(makeItem({ icon_path: '/custom/icon.png' })).iconPath).toBe('/custom/icon.png')
  })

  test('does not show sockets for gems or zero socket counts', () => {
    const gem = bankItemDetails(makeItem({
      affixes_json: '["Type: Gem", "Rarity: Normal"]',
      sockets: 3,
      class_req: 'Unknown',
      level_req: -1,
    }))
    expect(gem.attributes).toEqual([{ label: 'Level req.', value: 'Unknown' }])

    const unsocketed = bankItemDetails(makeItem({ sockets: 0 }))
    expect(unsocketed.attributes.some(({ label }) => label === 'Sockets')).toBe(false)
  })

  test('handles malformed metadata without throwing', () => {
    const details = bankItemDetails(makeItem({ affixes_json: '{invalid' }))
    expect(details.type).toBe('Uncategorized')
    expect(details.rarity).toBe('Unknown')
    expect(details.properties).toEqual([])
  })
})

describe('itemCompatibility', () => {
  test('distinguishes known usable, unmet, and unknown requirements', () => {
    expect(itemCompatibility(makeItem(), makeCharacter())).toBe('usable')
    expect(itemCompatibility(makeItem({ level_req: 31 }), makeCharacter())).toBe('unusable')
    expect(itemCompatibility(makeItem({ class_req: 'Sorceress' }), makeCharacter())).toBe('unusable')
    expect(itemCompatibility(makeItem({ level_req: -1 }), makeCharacter())).toBe('unknown')
    expect(itemCompatibility(makeItem({ class_req: 'Unknown' }), makeCharacter())).toBe('unknown')
    expect(itemCompatibility(makeItem(), undefined)).toBe('unknown')
  })
})

describe('filterBankItems', () => {
  const sword = makeItem()
  const transferredGem = makeItem({
    id: 2,
    base_item: 'Ruby',
    affixes_json: '["Type: Gem", "Rarity: Magic"]',
    class_req: 'Unknown',
    level_req: -1,
    sockets: 0,
    transferred_to: 'Hero',
  })
  const items = [sword, transferredGem]
  const details = new Map(items.map((item) => [item.id, bankItemDetails(item)]))
  const emptyFilters = {
    search: '', type: '', class: '', rarity: '', transfer: 'all' as const,
    compatibility: 'all' as const, minLevel: '', maxLevel: '', sockets: '',
  }

  test('combines search, type, transfer, compatibility and level filters', () => {
    expect(filterBankItems(items, details, {
      ...emptyFilters,
      search: 'fire skills',
      type: 'Weapon',
      transfer: 'available',
      compatibility: 'usable',
      minLevel: '20',
      maxLevel: '30',
    }, makeCharacter())).toEqual([sword])
  })

  test('filters transferred items and ignores unknown levels in level ranges', () => {
    expect(filterBankItems(items, details, {
      ...emptyFilters,
      transfer: 'transferred',
      minLevel: '1',
    }, undefined)).toEqual([])
  })

  test('matches three-letter rune names while typing', () => {
    const unrelatedSword = makeItem()
    const runes = [
      makeItem({ id: 3, base_item: 'Tir Rune', affixes_json: '["Type: Rune", "D2 item code: r03", "Rarity: Normal"]' }),
      makeItem({ id: 4, base_item: 'Tal Rune', affixes_json: '["Type: Rune", "D2 item code: r07", "Rarity: Normal"]' }),
      makeItem({ id: 5, base_item: 'Eth Rune', affixes_json: '["Type: Rune", "D2 item code: r05", "Rarity: Normal"]' }),
    ]
    const searchableItems = [unrelatedSword, ...runes]
    const runeDetails = new Map(searchableItems.map((item) => [item.id, bankItemDetails(item)]))

    for (const [query, expected] of [['tir', 'Tir Rune'], ['tal', 'Tal Rune'], ['eth', 'Eth Rune']]) {
      expect(filterBankItems(searchableItems, runeDetails, { ...emptyFilters, search: query }, undefined).map((item) => item.base_item)).toEqual([expected])
    }
    expect(filterBankItems(searchableItems, runeDetails, { ...emptyFilters, search: 'r03' }, undefined).map((item) => item.base_item)).toEqual(['Tir Rune'])
  })

  test('searches item adds in the regular query and the add filter', () => {
    const caster = makeItem({
      id: 6,
      affixes_json: JSON.stringify(['Type: Amulet', 'Rarity: Magic', 'Item Faster Cast Rate +10']),
    })
    const casterDetails = new Map([[caster.id, bankItemDetails(caster)]])

    expect(filterBankItems([caster], casterDetails, { ...emptyFilters, search: 'cast rate +10' }, undefined)).toEqual([caster])
    expect(filterBankItems([caster], casterDetails, { ...emptyFilters, search: 'fas' }, undefined)).toEqual([caster])
    expect(filterBankItems([caster], casterDetails, { ...emptyFilters, affix: 'Faster Cast Rate' }, undefined)).toEqual([caster])
    expect(filterBankItems([caster], casterDetails, { ...emptyFilters, affix: 'Life Steal' }, undefined)).toEqual([])
  })

  test('matches search terms at word boundaries instead of inside other words', () => {
    const damageItem = makeItem({
      id: 7,
      affixes_json: JSON.stringify(['Type: Weapon', 'Rarity: Normal', 'Maximum Damage +1']),
    })
    const mageItem = makeItem({
      id: 8,
      base_item: 'Mage Plate',
      affixes_json: JSON.stringify(['Type: Armor', 'Rarity: Normal']),
    })
    const searchableItems = [damageItem, mageItem]
    const itemDetails = new Map(searchableItems.map((item) => [item.id, bankItemDetails(item)]))

    expect(filterBankItems(searchableItems, itemDetails, { ...emptyFilters, search: 'Mage' }, undefined).map((item) => item.base_item)).toEqual(['Mage Plate'])
  })
})

describe('validateGoldDrafts', () => {
  test('accepts values at the carried and stash caps', () => {
    expect(validateGoldDrafts(99, '990000', '2500000')).toEqual({
      valid: true,
      carriedGold: 990000,
      stashGold: 2500000,
    })
  })

  test('rejects fractions, negatives and values over either cap', () => {
    expect(validateGoldDrafts(10, '100001', '0').valid).toBe(false)
    expect(validateGoldDrafts(10, '1.5', '0').valid).toBe(false)
    expect(validateGoldDrafts(10, '0', '2500001').valid).toBe(false)
  })
})
