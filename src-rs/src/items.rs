//! Diablo II item bit-stream helpers (shared by .d2s / .d2x parsers).

/// Read `n` bits LSB-first within each byte (Diablo II item encoding).
pub struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8], byte_offset: usize) -> Self {
        Self {
            data,
            bit: byte_offset * 8,
        }
    }

    pub fn read(&mut self, n: usize) -> Option<u64> {
        let mut v = 0u64;
        for i in 0..n {
            let byte_i = self.bit / 8;
            if byte_i >= self.data.len() {
                return None;
            }
            let bit = self.bit % 8;
            if (self.data[byte_i] >> bit) & 1 == 1 {
                v |= 1 << i;
            }
            self.bit += 1;
        }
        Some(v)
    }

    pub fn align_byte(&mut self) {
        let rem = self.bit % 8;
        if rem != 0 {
            self.bit += 8 - rem;
        }
    }

    pub fn byte_pos(&self) -> usize {
        self.bit.div_ceil(8)
    }
}

/// Minimal parsed item (enough for bank import of simple items like gems).
#[derive(Debug, Clone)]
pub struct ParsedItem {
    pub code: String,
    pub simple: bool,
    pub ethereal: bool,
    pub identified: bool,
    pub socketed: bool,
    pub quality: Option<u8>,
    pub parent: u8,
    pub equipped_slot: u8,
}

/// Parse one item starting at a `JM` header. Returns item + next byte offset.
pub fn parse_item_at(data: &[u8], offset: usize) -> Option<(ParsedItem, usize)> {
    if offset + 2 > data.len() || &data[offset..offset + 2] != b"JM" {
        return None;
    }

    let mut br = BitReader::new(data, offset);
    let header = br.read(16)?;
    if header != 0x4D4A {
        return None;
    }

    let _unk = br.read(4)?;
    let identified = br.read(1)? == 1;
    let _ = br.read(6)?;
    let socketed = br.read(1)? == 1;
    let _ = br.read(1)?;
    let _is_new = br.read(1)?;
    let _ = br.read(2)?;
    let is_ear = br.read(1)? == 1;
    let _starting = br.read(1)?;
    let _ = br.read(3)?;
    let simple = br.read(1)? == 1;
    let ethereal = br.read(1)? == 1;
    let _ = br.read(1)?;
    let _personalized = br.read(1)?;
    let _ = br.read(1)?;
    let _runeword = br.read(1)?;
    let _ = br.read(5)?;
    let _version = br.read(8)?;
    let _ = br.read(2)?;
    let parent = br.read(3)? as u8;
    let equipped_slot = br.read(4)? as u8;
    let _x = br.read(4)?;
    let _y = br.read(4)?;
    let _panel = br.read(3)?;

    if is_ear {
        let _class = br.read(3)?;
        let _level = br.read(7)?;
        loop {
            let c = br.read(7)?;
            if c == 0 {
                break;
            }
        }
        br.align_byte();
        return Some((
            ParsedItem {
                code: "ear".into(),
                simple: true,
                ethereal: false,
                identified: true,
                socketed: false,
                quality: None,
                parent,
                equipped_slot,
            },
            br.byte_pos(),
        ));
    }

    let mut chars = [0u8; 4];
    for c in &mut chars {
        *c = br.read(8)? as u8;
    }
    let code: String = chars
        .iter()
        .take_while(|&&c| c != 0 && c != b' ')
        .map(|&c| c as char)
        .collect();

    let _num_socketed = br.read(3)?;

    if simple {
        // Simple items are exactly 111 bits from the JM start, then padded.
        br.align_byte();
        return Some((
            ParsedItem {
                code,
                simple: true,
                ethereal,
                identified,
                socketed,
                quality: Some(2),
                parent,
                equipped_slot,
            },
            br.byte_pos(),
        ));
    }

    let quality = br.read(4).map(|value| value as u8);

    // Complex items: length varies. Find the next top-level JM after current byte.
    // Nested socketed JM items are still `JM` — for bank gem import we only need simples.
    br.align_byte();
    let search_from = br.byte_pos();
    let next = data[search_from..]
        .windows(2)
        .position(|w| w == b"JM")
        .map(|i| search_from + i)
        .unwrap_or(data.len());

    Some((
        ParsedItem {
            code,
            simple: false,
            ethereal,
            identified,
            socketed,
            quality,
            parent,
            equipped_slot,
        },
        next,
    ))
}

/// Base names for recognized item codes in supported stash imports.
/// Affixes and per-instance requirements are parsed separately from these names.
pub fn base_item_details(code: &str) -> Option<(&'static str, &'static str)> {
    Some(match code {
        "crn" => ("Crown", "Armor"),
        "amu" => ("Amulet", "Jewelry"),
        "chn" => ("Chain Mail", "Armor"),
        "rin" => ("Ring", "Jewelry"),
        "hbl" => ("Heavy Belt", "Armor"),
        "mbt" => ("Chain Boots", "Armor"),
        "pa1" => ("Targe", "Shield"),
        "xlg" => ("Demonhide Gloves", "Armor"),
        "gsc" => ("Grand Scepter", "Weapon"),
        "pa3" => ("Heraldic Shield", "Shield"),
        "qui" => ("Quilted Armor", "Armor"),
        "lbl" => ("Sash", "Armor"),
        "cap" => ("Cap", "Armor"),
        "lbt" => ("Leather Boots", "Armor"),
        "tkf" => ("Throwing Knife", "Weapon"),
        "buc" => ("Buckler", "Shield"),
        "r01" => ("El Rune", "Rune"),
        "gpr" => ("Perfect Ruby", "Gem"),
        "cm1" => ("Small Charm", "Charm"),
        "cm3" => ("Grand Charm", "Charm"),
        "jew" => ("Jewel", "Jewel"),
        "tow" => ("Tower Shield", "Shield"),
        "2ax" => ("Double Axe", "Weapon"),
        "lbw" => ("Long Bow", "Weapon"),
        "rvs" => ("Rejuvenation Potion", "Potion"),
        _ => return None,
    })
}

/// Human-readable names for common gem / rune / skull codes.
pub fn item_display_name(code: &str) -> String {
    match code {
        // Amethyst
        "gcv" => "Chipped Amethyst",
        "gfv" => "Flawed Amethyst",
        "gsv" => "Amethyst",
        "gzv" => "Flawless Amethyst",
        "gpv" => "Perfect Amethyst",
        // Topaz
        "gcy" => "Chipped Topaz",
        "gfy" => "Flawed Topaz",
        "gsy" => "Topaz",
        "gly" => "Flawless Topaz",
        "gpy" => "Perfect Topaz",
        // Sapphire
        "gcb" => "Chipped Sapphire",
        "gfb" => "Flawed Sapphire",
        "gsb" => "Sapphire",
        "glb" => "Flawless Sapphire",
        "gpb" => "Perfect Sapphire",
        // Emerald
        "gcg" => "Chipped Emerald",
        "gfg" => "Flawed Emerald",
        "gsg" => "Emerald",
        "glg" => "Flawless Emerald",
        "gpg" => "Perfect Emerald",
        // Ruby
        "gcr" => "Chipped Ruby",
        "gfr" => "Flawed Ruby",
        "gsr" => "Ruby",
        "glr" => "Flawless Ruby",
        "gpr" => "Perfect Ruby",
        // Diamond
        "gcw" => "Chipped Diamond",
        "gfw" => "Flawed Diamond",
        "gsw" => "Diamond",
        "glw" => "Flawless Diamond",
        "gpw" => "Perfect Diamond",
        // Skull
        "skc" => "Chipped Skull",
        "skf" => "Flawed Skull",
        "sku" => "Skull",
        "skl" => "Flawless Skull",
        "skz" => "Perfect Skull",
        // Supported rune codes
        "r01" => "El Rune",
        "r02" => "Eld Rune",
        "r03" => "Tir Rune",
        "r04" => "Nef Rune",
        "r05" => "Eth Rune",
        "r06" => "Ith Rune",
        "r07" => "Tal Rune",
        "r08" => "Ral Rune",
        "r09" => "Ort Rune",
        "r10" => "Thul Rune",
        "r11" => "Amn Rune",
        "r12" => "Sol Rune",
        "r13" => "Shael Rune",
        "r14" => "Dol Rune",
        "r15" => "Hel Rune",
        other => other,
    }
    .to_string()
}

pub fn item_quality_name(quality: Option<u8>) -> &'static str {
    match quality {
        Some(1) => "Low quality",
        Some(2) => "Normal",
        Some(3) => "Superior",
        Some(4) => "Magic",
        Some(5) => "Set",
        Some(6) => "Rare",
        Some(7) => "Unique",
        Some(8) => "Crafted",
        Some(9) => "Tempered",
        _ => "Unknown",
    }
}

pub fn is_gem_code(code: &str) -> bool {
    matches!(
        code,
        "gcv"
            | "gfv"
            | "gsv"
            | "gzv"
            | "gpv"
            | "gcy"
            | "gfy"
            | "gsy"
            | "gly"
            | "gpy"
            | "gcb"
            | "gfb"
            | "gsb"
            | "glb"
            | "gpb"
            | "gcg"
            | "gfg"
            | "gsg"
            | "glg"
            | "gpg"
            | "gcr"
            | "gfr"
            | "gsr"
            | "glr"
            | "gpr"
            | "gcw"
            | "gfw"
            | "gsw"
            | "glw"
            | "gpw"
            | "skc"
            | "skf"
            | "sku"
            | "skl"
            | "skz"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct BitFixture {
        bytes: Vec<u8>,
        bit: usize,
    }

    impl BitFixture {
        fn push(&mut self, value: u64, width: usize) {
            for offset in 0..width {
                if self.bit / 8 == self.bytes.len() {
                    self.bytes.push(0);
                }
                if (value >> offset) & 1 == 1 {
                    self.bytes[self.bit / 8] |= 1 << (self.bit % 8);
                }
                self.bit += 1;
            }
        }

        fn simple_item(code: &[u8; 4]) -> Vec<u8> {
            let mut fixture = Self::default();
            fixture.push(0x4D4A, 16); // JM
            fixture.push(0, 4); // unknown
            fixture.push(1, 1); // identified
            fixture.push(0, 6); // unknown
            fixture.push(0, 1); // socketed
            fixture.push(0, 1); // unknown
            fixture.push(0, 1); // new
            fixture.push(0, 2); // unknown
            fixture.push(0, 1); // ear
            fixture.push(0, 1); // starting item
            fixture.push(0, 3); // unknown
            fixture.push(1, 1); // simple
            fixture.push(1, 1); // ethereal
            fixture.push(0, 1); // unknown
            fixture.push(0, 1); // personalized
            fixture.push(0, 1); // unknown
            fixture.push(0, 1); // runeword
            fixture.push(0, 5); // unknown
            fixture.push(0x60, 8); // item version
            fixture.push(0, 2); // unknown
            fixture.push(3, 3); // parent
            fixture.push(2, 4); // equipped slot
            fixture.push(4, 4); // x
            fixture.push(5, 4); // y
            fixture.push(1, 3); // panel
            for byte in code {
                fixture.push(*byte as u64, 8);
            }
            fixture.push(0, 3); // socketed item count
            fixture.bytes
        }
    }

    #[test]
    fn bit_reader_reads_lsb_first_and_respects_byte_offsets() {
        let bytes = [0b1011_0100, 0b0000_0011];
        let mut reader = BitReader::new(&bytes, 0);
        assert_eq!(reader.read(4), Some(0b0100));
        assert_eq!(reader.read(4), Some(0b1011));
        assert_eq!(reader.read(2), Some(0b11));
        assert_eq!(reader.read(1), Some(0));
        assert_eq!(reader.read(1), Some(0));

        let mut offset_reader = BitReader::new(&bytes, 1);
        assert_eq!(offset_reader.read(2), Some(0b11));
    }

    #[test]
    fn parses_simple_item_fields_and_aligns_to_next_byte() {
        let mut bytes = BitFixture::simple_item(b"r01 ");
        bytes.extend_from_slice(b"NEXT");
        let (item, next) = parse_item_at(&bytes, 0).expect("valid simple item");

        assert_eq!(item.code, "r01");
        assert!(item.simple);
        assert!(item.identified);
        assert!(item.ethereal);
        assert!(!item.socketed);
        assert_eq!(item.parent, 3);
        assert_eq!(item.equipped_slot, 2);
        assert_eq!(&bytes[next..next + 4], b"NEXT");
    }

    #[test]
    fn rejects_non_item_and_truncated_item_records() {
        assert!(parse_item_at(b"NO", 0).is_none());
        assert!(parse_item_at(b"JM", 0).is_none());
        assert!(parse_item_at(b"xJM", 1).is_none());
    }

    #[test]
    fn classifies_item_codes_and_quality_labels() {
        assert!(is_gem_code("gpr"));
        assert!(is_gem_code("skz"));
        assert!(!is_gem_code("r01"));
        assert_eq!(item_display_name("r03"), "Tir Rune");
        assert_eq!(item_quality_name(Some(7)), "Unique");
        assert_eq!(item_quality_name(None), "Unknown");
        assert_eq!(base_item_details("amu"), Some(("Amulet", "Jewelry")));
    }
}
