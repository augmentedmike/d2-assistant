//! Diablo II stash (.d2x) binary parser.
//!
//! Reads classic Diablo II version 96 `.d2x` stash files.

use crate::items::{ParsedItem, is_gem_code, item_display_name, item_quality_name, parse_item_at};
use d2s::{
    Item, TxtData,
    bits::{BitReader, BitWriter},
    data::GameTable,
};
use serde::Serialize;
use std::fs;

/// Parsed shared stash data from a .d2x file
#[derive(Debug, Clone, Serialize)]
pub struct StashData {
    pub file_size: u32,
    pub item_count: u16,
    pub gold: u32,
    pub format: String,
    pub items: Vec<StashItemSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StashItemSummary {
    pub code: String,
    pub name: String,
    pub simple: bool,
    pub is_gem: bool,
    pub identified: bool,
    pub ethereal: bool,
    pub sockets: i32,
    pub quality: String,
}

/// Error type for D2X parsing
#[derive(Debug, thiserror::Error)]
pub enum D2xError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported stash format (expected D2X header)")]
    UnsupportedFormat,
}

fn parse_d2x_items(data: &[u8], expected_count: u16) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let mut pos = 0x0B;
    let mut guard = 0usize;
    while pos + 2 <= data.len() && items.len() < expected_count as usize + 64 {
        if &data[pos..pos + 2] != b"JM" {
            match data[pos..].windows(2).position(|w| w == b"JM") {
                Some(i) => pos += i,
                None => break,
            }
        }
        match parse_item_at(data, pos) {
            Some((item, next)) => {
                pos = next.max(pos + 2);
                items.push(item);
            }
            None => break,
        }
        guard += 1;
        if guard > 10_000 {
            break;
        }
    }
    items
}

#[derive(Debug, Clone)]
pub struct TransferableStashItem {
    pub code: String,
    pub seed: u32,
    pub quality: String,
    pub identified: bool,
    pub ethereal: bool,
    pub sockets: i32,
    pub level_req: i32,
    pub class_req: String,
    pub stats: Vec<(u16, i64, i32)>,
    pub item_data: Vec<u8>,
}

pub fn d2x_checksum(data: &[u8]) -> Result<u32, String> {
    if data.len() < 11 || &data[..3] != b"D2X" {
        return Err("not a D2X stash".into());
    }
    Ok(data.iter().enumerate().fold(0u32, |sum, (index, byte)| {
        let value = if (7..11).contains(&index) {
            0
        } else {
            *byte as u32
        };
        sum.wrapping_shl(1)
            .wrapping_add(value)
            .wrapping_add(sum >> 31)
    }))
}

pub fn empty_d2x_stash() -> Vec<u8> {
    let mut data = vec![b'D', b'2', b'X', 0, 0, 96, 0, 0, 0, 0, 0];
    let checksum = d2x_checksum(&data).unwrap_or_default();
    data[7..11].copy_from_slice(&checksum.to_le_bytes());
    data
}

/// Read every v96 item and retain its complete, re-encoded item record for later export.
pub fn read_transferable_stash_items(data: &[u8]) -> Result<Vec<TransferableStashItem>, String> {
    if data.len() < 11 || &data[..3] != b"D2X" {
        return Err("not a D2X stash".into());
    }
    let count = u16::from_le_bytes([data[3], data[4]]) as usize;
    let version = u16::from_le_bytes([data[5], data[6]]);
    if version != 96 {
        return Err(format!("unsupported D2X item version: {version}"));
    }
    let expected_checksum = u32::from_le_bytes(data[7..11].try_into().unwrap());
    let calculated_checksum = d2x_checksum(data)?;
    if expected_checksum != calculated_checksum {
        return Err(format!(
            "stash checksum mismatch: file has {expected_checksum:08x}, calculated {calculated_checksum:08x}"
        ));
    }

    let txt = TxtData::embedded();
    let mut reader = BitReader::new(&data[11..]);
    let mut items = Vec::with_capacity(count);
    for index in 0..count {
        let item = Item::read(&mut reader, txt, 96)
            .map_err(|error| format!("could not decode stash item {index}: {error}"))?;
        let mut item_data = Vec::new();
        item.write(&mut BitWriter::new(&mut item_data), txt, 96)
            .map_err(|error| format!("could not encode stash item {index}: {error}"))?;
        items.push(TransferableStashItem {
            code: item.code_string(),
            seed: item.seed,
            quality: format!("{:?}", item.quality),
            identified: item.flags.contains(d2s::ItemFlags::IDENTIFIED),
            ethereal: item.flags.contains(d2s::ItemFlags::ETHEREAL),
            sockets: item.sockets.len() as i32,
            level_req: item_level_requirement(
                &item.code_string(),
                &item.quality,
                item.flags.contains(d2s::ItemFlags::RUNEWORD),
                txt,
            ),
            class_req: item_class_requirement(&item.code_string(), txt),
            stats: item
                .stats
                .iter()
                .map(|stat| (stat.id.0, stat.value, stat.layer))
                .collect(),
            item_data,
        });
    }
    Ok(items)
}

pub fn requirements_from_item_record(item_data: &[u8]) -> Result<(i32, String), String> {
    let txt = TxtData::embedded();
    let mut reader = BitReader::new(item_data);
    let item = Item::read(&mut reader, txt, 96)
        .map_err(|error| format!("could not decode item requirements: {error}"))?;
    let code = item.code_string();
    Ok((
        item_level_requirement(
            &code,
            &item.quality,
            item.flags.contains(d2s::ItemFlags::RUNEWORD),
            txt,
        ),
        item_class_requirement(&code, txt),
    ))
}

fn item_table_row<'a>(code: &str, txt: &'a TxtData) -> Option<(Vec<&'a str>, Vec<&'a str>)> {
    for table in [GameTable::Weapons, GameTable::Armor, GameTable::Misc] {
        let contents = txt.table(table, 96).ok()?;
        let mut rows = contents.lines();
        let headers: Vec<_> = rows.next()?.split('\t').collect();
        let code_column = headers.iter().position(|header| *header == "code")?;
        for row in rows {
            let values: Vec<_> = row.split('\t').collect();
            if values.get(code_column).is_some_and(|value| *value == code) {
                return Some((headers, values));
            }
        }
    }
    None
}

pub(crate) fn item_level_requirement(
    code: &str,
    quality: &d2s::ItemQuality,
    runeword: bool,
    txt: &TxtData,
) -> i32 {
    if runeword
        || !matches!(
            quality,
            d2s::ItemQuality::Inferior | d2s::ItemQuality::Normal | d2s::ItemQuality::Superior
        )
    {
        return -1;
    }
    let Some((headers, values)) = item_table_row(code, txt) else {
        return -1;
    };
    let Some(column) = headers.iter().position(|header| *header == "levelreq") else {
        return -1;
    };
    values
        .get(column)
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(-1)
}

pub(crate) fn item_class_requirement(code: &str, txt: &TxtData) -> String {
    let Some((headers, values)) = item_table_row(code, txt) else {
        return "Unknown".into();
    };
    let type_columns = ["type", "type2"]
        .iter()
        .filter_map(|name| {
            headers
                .iter()
                .position(|header| header.eq_ignore_ascii_case(name))
        })
        .filter_map(|column| values.get(column).copied())
        .filter(|value| !value.is_empty());
    let Ok(types) = txt.table(GameTable::ItemTypes, 96) else {
        return "Unknown".into();
    };
    let mut type_rows = types.lines();
    let Some(type_headers) = type_rows.next() else {
        return "Unknown".into();
    };
    let type_headers: Vec<_> = type_headers.split('\t').collect();
    let Some(code_column) = type_headers.iter().position(|header| *header == "Code") else {
        return "Unknown".into();
    };
    let Some(class_column) = type_headers.iter().position(|header| *header == "Class") else {
        return "Unknown".into();
    };
    for item_type in type_columns {
        for row in type_rows.clone() {
            let fields: Vec<_> = row.split('\t').collect();
            if fields
                .get(code_column)
                .is_some_and(|value| *value == item_type)
                && let Some(class_code) = fields.get(class_column)
            {
                let class = match *class_code {
                    "ama" => "Amazon",
                    "bar" => "Barbarian",
                    "nec" => "Necromancer",
                    "pal" => "Paladin",
                    "sor" => "Sorceress",
                    "ass" => "Assassin",
                    "dru" => "Druid",
                    _ => "Any",
                };
                if class != "Any" {
                    return class.into();
                }
            }
        }
    }
    "Any".into()
}

/// Parse a supported version 96 .d2x stash file.
pub fn parse_d2x(path: &str) -> Result<StashData, D2xError> {
    let data = fs::read(path).map_err(D2xError::Io)?;
    let file_size = data.len() as u32;

    if data.len() < 11 {
        return Ok(StashData {
            file_size,
            item_count: 0,
            gold: 0,
            format: "empty_or_unknown".into(),
            items: vec![],
        });
    }

    // "D2X" + u16 count + u16 version(96) + u32 checksum + items @ 0x0B
    if &data[0..3] == b"D2X" {
        let item_count = u16::from_le_bytes([data[3], data[4]]);
        let version = u16::from_le_bytes([data[5], data[6]]);
        if version != 96 {
            return Err(D2xError::UnsupportedFormat);
        }
        let parsed = parse_d2x_items(&data, item_count);
        let items: Vec<StashItemSummary> = parsed
            .into_iter()
            .map(|p| StashItemSummary {
                is_gem: is_gem_code(&p.code),
                name: item_display_name(&p.code),
                code: p.code,
                simple: p.simple,
                identified: p.identified,
                ethereal: p.ethereal,
                sockets: if p.socketed { -1 } else { 0 },
                quality: item_quality_name(p.quality).into(),
            })
            .collect();

        return Ok(StashData {
            file_size,
            item_count,
            gold: 0,
            format: "d2x".into(),
            items,
        });
    }

    Ok(StashData {
        file_size,
        item_count: 0,
        gold: 0,
        format: "unknown".into(),
        items: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Optional local integration check. Supply your own supported v96 stash:
    /// D2A_TEST_STASH=/path/to/stash.d2x cargo test --manifest-path src-rs/Cargo.toml -- --ignored
    #[test]
    #[ignore = "requires a local stash save; private fixtures are not distributed"]
    fn parse_local_stash_fixture() {
        let path =
            std::env::var("D2A_TEST_STASH").expect("set D2A_TEST_STASH to a local .d2x file");
        let data = parse_d2x(&path).expect("stash should parse");
        assert!(data.file_size > 0);
    }
}
