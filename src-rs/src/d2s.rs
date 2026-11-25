//! Diablo II character file (.d2s) binary parser.
//!
//! Reads classic Diablo II (1.13d + LOD) character save files.
//! Original files are never modified — this is read-only.

use serde::Serialize;
use std::fs;
use std::io::{Cursor, Read};

use crate::items::{BitReader, base_item_details, parse_item_at};

/// Diablo II character classes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum CharClass {
    Amazon = 0,
    Sorceress = 1,
    Necromancer = 2,
    Paladin = 3,
    Barbarian = 4,
    Druid = 5,
    Assassin = 6,
    Unknown(u8),
}

impl From<u8> for CharClass {
    fn from(b: u8) -> Self {
        match b {
            0 => CharClass::Amazon,
            1 => CharClass::Sorceress,
            2 => CharClass::Necromancer,
            3 => CharClass::Paladin,
            4 => CharClass::Barbarian,
            5 => CharClass::Druid,
            6 => CharClass::Assassin,
            other => CharClass::Unknown(other),
        }
    }
}

/// Parsed character data from a .d2s file
#[derive(Debug, Clone, Serialize)]
pub struct CharacterData {
    pub name: String,
    pub class: CharClass,
    pub level: u8,
    pub save_version: u32,
    pub experience: u32,
    pub strength: u16,
    pub dexterity: u16,
    pub vitality: u16,
    pub energy: u16,
    pub life: u16,
    pub mana: u16,
    pub stamina: u16,
    pub gold: u32,
    pub stash_gold: u32,
    pub stat_points: u16,
    pub skill_points: u16,
    pub file_size: u32,
    /// Set to true when stat offsets have been validated against real files
    pub stats_validated: bool,
    pub equipped_items: Vec<EquippedItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EquippedItem {
    pub code: String,
    pub name: String,
    pub item_type: String,
    pub slot: String,
    pub identified: bool,
    pub ethereal: bool,
    pub socketed: bool,
}

#[derive(Default)]
struct ParsedStats {
    strength: u16,
    dexterity: u16,
    vitality: u16,
    energy: u16,
    life: u16,
    mana: u16,
    stamina: u16,
    gold: u32,
    stash_gold: u32,
    stat_points: u16,
    skill_points: u16,
    experience: u32,
    level: Option<u8>,
}

fn stat_bit_width(id: u64) -> Option<usize> {
    match id {
        0..=4 => Some(10),
        5 => Some(8),
        6..=11 => Some(21),
        12 => Some(7),
        13 => Some(32),
        14..=15 => Some(25),
        _ => None,
    }
}

fn parse_stats(data: &[u8], header_level: u8) -> Option<(ParsedStats, usize)> {
    const STATS_OFFSET: usize = 0x2FD;
    if data.get(STATS_OFFSET..STATS_OFFSET + 2)? != b"gf" {
        return None;
    }

    let mut bits = BitReader::new(data, STATS_OFFSET + 2);
    let mut stats = ParsedStats::default();
    let mut found_level = false;
    let mut terminated = false;

    for _ in 0..64 {
        let id = bits.read(9)?;
        if id == 0x1ff {
            terminated = true;
            break;
        }
        let width = stat_bit_width(id)?;
        let raw = bits.read(width)? as u32;
        match id {
            0 => stats.strength = raw as u16,
            1 => stats.energy = raw as u16,
            2 => stats.dexterity = raw as u16,
            3 => stats.vitality = raw as u16,
            4 => stats.stat_points = raw as u16,
            5 => stats.skill_points = raw as u16,
            6 | 7 => stats.life = (raw / 256).min(u16::MAX as u32) as u16,
            8 | 9 => stats.mana = (raw / 256).min(u16::MAX as u32) as u16,
            10 | 11 => stats.stamina = (raw / 256).min(u16::MAX as u32) as u16,
            12 => {
                stats.level = Some(raw as u8);
                found_level = raw as u8 == header_level;
            }
            13 => stats.experience = raw,
            14 => stats.gold = raw,
            15 => stats.stash_gold = raw,
            _ => {}
        }
    }

    if terminated && found_level {
        Some((stats, bits.byte_pos()))
    } else {
        None
    }
}

fn equipment_slot_name(slot: u8) -> &'static str {
    match slot {
        1 => "Head",
        2 => "Amulet",
        3 => "Armor",
        4 => "Right hand",
        5 => "Left hand",
        6 => "Right ring",
        7 => "Left ring",
        8 => "Belt",
        9 => "Boots",
        10 => "Gloves",
        11 => "Swap right hand",
        12 => "Swap left hand",
        _ => "Unknown slot",
    }
}

fn parse_equipped_items(data: &[u8], stats_end: usize) -> Vec<EquippedItem> {
    if data.get(stats_end..stats_end + 2) != Some(b"if") {
        return vec![];
    }
    let list_header = stats_end + 32;
    if data.get(list_header..list_header + 2) != Some(b"JM") {
        return vec![];
    }
    let items_start = list_header + 4;
    let items_end = data[items_start..]
        .windows(2)
        .position(|marker| marker == b"jf")
        .map(|offset| items_start + offset)
        .unwrap_or(data.len());

    let mut equipped = Vec::new();
    for offset in items_start..items_end.saturating_sub(1) {
        if data.get(offset..offset + 2) != Some(b"JM") {
            continue;
        }
        let Some((item, _)) = parse_item_at(data, offset) else {
            continue;
        };
        if item.parent != 1 || !(1..=12).contains(&item.equipped_slot) {
            continue;
        }
        let Some((name, item_type)) = base_item_details(&item.code) else {
            continue;
        };
        if equipped.iter().any(|existing: &EquippedItem| {
            existing.code == item.code && existing.slot == equipment_slot_name(item.equipped_slot)
        }) {
            continue;
        }
        equipped.push(EquippedItem {
            code: item.code,
            name: name.into(),
            item_type: item_type.into(),
            slot: equipment_slot_name(item.equipped_slot).into(),
            identified: item.identified,
            ethereal: item.ethereal,
            socketed: item.socketed,
        });
    }
    equipped
}

/// Error type for D2S parsing
#[derive(Debug, thiserror::Error)]
pub enum D2sError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid signature: expected 0xAA55AA55, got 0x{0:08X}")]
    InvalidSignature(u32),
    #[error("file too short: {0} bytes, expected at least {1}")]
    TooShort(usize, usize),
    #[error("invalid name encoding")]
    InvalidName,
}

/// Read a little-endian u32 from a cursor
fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, std::io::Error> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

/// Read a little-endian u16 from a cursor
fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, std::io::Error> {
    let mut buf = [0u8; 2];
    cursor.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

/// Read a little-endian u8 from a cursor
fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, std::io::Error> {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf)?;
    Ok(buf[0])
}

/// Parse a .d2s character file
pub fn parse_d2s(path: &str) -> Result<CharacterData, D2sError> {
    let data = fs::read(path).map_err(D2sError::Io)?;

    if data.len() < 0x30 {
        return Err(D2sError::TooShort(data.len(), 0x30));
    }

    let mut cursor = Cursor::new(data.as_slice());

    // Header layout (v1.10+ / version id 0x60) — see krisives/d2s-format
    // 0x00 signature, 0x04 version, 0x08 file size, 0x0C checksum,
    // 0x10 active weapon, 0x14 name[16], 0x24 status, 0x25 progression,
    // 0x26 unk[2], 0x28 class, 0x29 unk[2], 0x2B level, …

    // Signature: 0xAA55AA55 (stored as 55 AA 55 AA in little-endian)
    let signature = read_u32(&mut cursor)?;
    if signature != 0xAA55AA55 {
        return Err(D2sError::InvalidSignature(signature));
    }

    // Offset 0x04: file version (0x60 = 1.10+)
    let save_version = read_u32(&mut cursor)?;

    // Offset 0x08: file size
    let file_size = read_u32(&mut cursor)?;

    // Offset 0x0C: checksum (skip)
    let _checksum = read_u32(&mut cursor)?;

    // Offset 0x10: active weapon set
    let _active_weapon = read_u32(&mut cursor)?;

    // Offset 0x14: character name (16 bytes, null-padded)
    let mut name_bytes = [0u8; 16];
    cursor.read_exact(&mut name_bytes)?;
    let name = String::from_utf8(
        name_bytes
            .iter()
            .take_while(|&&b| b != 0)
            .copied()
            .collect(),
    )
    .map_err(|_| D2sError::InvalidName)?;

    // Offset 0x24: status / progression (not experience — that is bit-packed in stats)
    cursor.set_position(0x24);
    let _status = read_u8(&mut cursor)?;
    let _progression = read_u8(&mut cursor)?;
    let _unk_26 = read_u16(&mut cursor)?;

    // Offset 0x28: class
    let class_byte = read_u8(&mut cursor)?;
    let class = CharClass::from(class_byte);

    // Offset 0x29: unknown (commonly 0x10, 0x1E) — NOT level
    let _unk_29 = read_u16(&mut cursor)?;

    // Offset 0x2B: character level (character-select screen value)
    let level = read_u8(&mut cursor)?;

    // Attributes and experience are 9-bit stat ids followed by values of
    // per-stat widths. Vitals use an 8-bit fractional component in the save.
    let parsed_stats = parse_stats(&data, level);
    let stats_validated = parsed_stats.is_some();
    let (stats, stats_end) = parsed_stats.unwrap_or_default();
    let equipped_items = if stats_validated {
        parse_equipped_items(&data, stats_end)
    } else {
        vec![]
    };

    Ok(CharacterData {
        name,
        class,
        level,
        save_version,
        experience: stats.experience,
        strength: stats.strength,
        dexterity: stats.dexterity,
        vitality: stats.vitality,
        energy: stats.energy,
        life: stats.life,
        mana: stats.mana,
        stamina: stats.stamina,
        gold: stats.gold,
        stash_gold: stats.stash_gold,
        stat_points: stats.stat_points,
        skill_points: stats.skill_points,
        file_size,
        stats_validated,
        equipped_items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Optional local integration check. Supply your own private v96-era save:
    /// D2A_TEST_CHARACTER=/path/to/character.d2s cargo test --manifest-path src-rs/Cargo.toml -- --ignored
    #[test]
    #[ignore = "requires a local character save; private fixtures are not distributed"]
    fn parse_local_character_fixture() {
        let path = std::env::var("D2A_TEST_CHARACTER")
            .expect("set D2A_TEST_CHARACTER to a local .d2s file");
        let data = parse_d2s(&path).expect("character save should parse");
        assert!(data.file_size > 0);
    }
}
