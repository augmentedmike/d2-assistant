import classicItemIcons from './data/classic-item-icons-v96.json'

const iconMap = classicItemIcons as Record<string, string>

export function classicItemIcon(code: string): string | null {
  const icon = iconMap[code.toLowerCase()]
  return icon ? `/item-icons/${icon}.png` : null
}
