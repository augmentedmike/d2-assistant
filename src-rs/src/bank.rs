//! Equipment bank data model.
//!
//! Defines the BankItem struct used by the Tauri commands in lib.rs.
//! Database operations are performed directly in lib.rs commands.

use serde::Serialize;

/// An item in the equipment bank
#[derive(Debug, Clone, Serialize)]
pub struct BankItem {
    pub id: i64,
    pub base_item: String,
    pub affixes_json: String,
    pub level_req: i32,
    pub class_req: String,
    pub sockets: i32,
    pub icon_path: Option<String>,
    pub source_character: String,
    pub banked_at: i64,
    pub updated_at: i64,
    pub has_item_data: bool,
    pub transferred_to: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bank_item_serialization() {
        let item = BankItem {
            id: 1,
            base_item: "Stone of Jordan".to_string(),
            affixes_json: "[\"+1 to All Skills\"]".to_string(),
            level_req: 65,
            class_req: "Any".to_string(),
            sockets: 0,
            icon_path: None,
            source_character: "Example Character".to_string(),
            banked_at: 1000,
            updated_at: 1000,
            has_item_data: false,
            transferred_to: None,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("Stone of Jordan"));
        assert!(json.contains("Example Character"));
    }
}
