//! Key usage: how often each key (by layer and matrix position, never by
//! keycode) was pressed, in hourly buckets. The keyboard keeps counters in RAM
//! and hands them over, zeroed, in chunks of `event_USAGE` packets.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use rusqlite::Connection;
use rusqlite::params;

use serde::Deserialize;

use crate::labels::KEY_LABELS;
use crate::layout::KEY_INFO;
use crate::layout::KEYS;
use crate::layout::finger;
use crate::protocol::REPORT_LEN;
use crate::protocol::Report;

pub const LAYERS: [&str; 4] = ["qwerty", "symbols", "numbers", "media"];
const ROWS: usize = 12;
const COLS: usize = 7;
const PER_CHUNK: usize = 15;
const MODS: [&str; 4] = ["ctrl", "shift", "alt", "gui"];
const DUAL_KINDS: [&str; 3] = ["tap", "hold", "misfire"];
/// The layer time counters count this many milliseconds.
const LAYER_TIME_UNIT_MS: u64 = 100;

/// The counter tables the keyboard keeps and hands over zeroed, in chunks of packets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Table {
    /// Presses per (layer, row, col).
    Keys,
    /// Presses made while a modifier is held, per (modifier mask, row, col).
    Mods,
    /// Taps, holds and misfires per (kind, row, col) of dual-role keys.
    Dual,
    /// Entries per layer, then time on top per layer.
    Layers,
}

impl Table {
    pub const ALL: [Table; 4] = [Table::Keys, Table::Mods, Table::Dual, Table::Layers];

    fn request(self) -> u8 {
        match self {
            Table::Keys => 12,
            Table::Mods => 14,
            Table::Dual => 16,
            Table::Layers => 18,
        }
    }

    fn total(self) -> usize {
        match self {
            Table::Keys => LAYERS.len() * ROWS * COLS,
            Table::Mods => 16 * ROWS * COLS,
            Table::Dual => DUAL_KINDS.len() * ROWS * COLS,
            Table::Layers => 2 * LAYERS.len(),
        }
    }

    pub fn chunks(self) -> u8 {
        self.total().div_ceil(PER_CHUNK) as u8
    }

    fn from_event(event: u8) -> Option<Table> {
        Table::ALL
            .into_iter()
            .find(|table| table.request() + 1 == event)
    }

    /// The table and chunk to ask for after `chunk` of this one.
    pub fn next(
        self,
        chunk: u8,
    ) -> Option<(Table, u8)> {
        if chunk + 1 < self.chunks() {
            return Some((self, chunk + 1));
        }
        let position = Table::ALL.iter().position(|table| *table == self)?;
        Table::ALL.get(position + 1).map(|table| (*table, 0))
    }
}

/// Presses of one key during one hour (hours since the epoch).
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct Bucket {
    pub hour: u64,
    pub layer: u8,
    pub row: u8,
    pub col: u8,
    pub count: u64,
}

/// Hours since the epoch.
pub fn hour_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / 3600)
}

pub fn request_report(
    table: Table,
    chunk: u8,
) -> Report {
    let mut report = [0; REPORT_LEN];
    report[0] = table.request();
    report[1] = chunk;
    report
}

/// The non-zero counts of a counters packet as (index, count), with the table they belong to.
pub fn decode(packet: &Report) -> Option<(Table, Vec<(usize, u16)>)> {
    let table = Table::from_event(packet[0])?;
    let first = usize::from(packet[1]) * PER_CHUNK;
    let counts = packet[2..]
        .chunks_exact(2)
        .enumerate()
        .map(|(i, pair)| (first + i, u16::from_le_bytes([pair[0], pair[1]])))
        .filter(|&(index, count)| count > 0 && index < table.total())
        .collect();
    Some((table, counts))
}

/// A modifier mask as "ctrl+shift".
fn mods_name(mask: usize) -> String {
    let names: Vec<&str> = MODS
        .iter()
        .enumerate()
        .filter(|(bit, _)| mask >> bit & 1 == 1)
        .map(|(_, name)| *name)
        .collect();
    names.join("+")
}

/// The database Grafana reads: the same path on the host and in the container.
pub fn path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join("claude/.services/keyboard/usage.sqlite"))
}

fn legacy_path() -> Option<PathBuf> {
    dirs::state_dir().map(|dir| dir.join("moonlander-leds").join("key-usage.json"))
}

/// Open (creating if needed) the database at `path`, with the tables and the static key layout.
pub fn open(path: &Path) -> anyhow::Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS key_usage (
             hour INTEGER, layer TEXT, row INTEGER, col INTEGER, count INTEGER,
             PRIMARY KEY (hour, layer, row, col));
         CREATE TABLE IF NOT EXISTS mod_usage (
             hour INTEGER, mods TEXT, row INTEGER, col INTEGER, count INTEGER,
             PRIMARY KEY (hour, mods, row, col));
         CREATE TABLE IF NOT EXISTS dualrole_usage (
             hour INTEGER, row INTEGER, col INTEGER, kind TEXT, count INTEGER,
             PRIMARY KEY (hour, row, col, kind));
         CREATE TABLE IF NOT EXISTS layer_usage (
             hour INTEGER, layer TEXT, entries INTEGER, held_ms INTEGER,
             PRIMARY KEY (hour, layer));
         DROP TABLE IF EXISTS key_layout;
         CREATE TABLE key_layout (
             row INTEGER, col INTEGER, half TEXT, finger TEXT, x REAL, y REAL, label TEXT,
             PRIMARY KEY (row, col));
         DROP TABLE IF EXISTS key_label;
         CREATE TABLE key_label (
             layer TEXT, row INTEGER, col INTEGER, label TEXT,
             PRIMARY KEY (layer, row, col));",
    )?;
    let tx = conn.unchecked_transaction()?;
    for &(row, col, x, y, label) in &KEY_INFO {
        let half = if row < 6 { "left" } else { "right" };
        tx.execute(
            "INSERT INTO key_layout VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![row, col, half, finger(row, x), x, y, label],
        )?;
    }
    for &(layer, row, col, label) in &KEY_LABELS {
        tx.execute(
            "INSERT INTO key_label VALUES (?1, ?2, ?3, ?4)",
            params![layer, row, col, label],
        )?;
    }
    tx.commit()?;
    Ok(conn)
}

/// Open the default database and move the old JSON buckets into it, once.
pub fn open_default() -> anyhow::Result<Connection> {
    let conn = open(&path().ok_or(anyhow::anyhow!("No home directory"))?)?;
    if let Some(old) = legacy_path().filter(|old| old.exists()) {
        let buckets: Vec<Bucket> = serde_json::from_str(&std::fs::read_to_string(&old)?)?;
        let tx = conn.unchecked_transaction()?;
        for bucket in &buckets {
            let name = LAYERS
                .get(usize::from(bucket.layer))
                .copied()
                .unwrap_or("?");
            upsert(&tx, bucket.hour, name, bucket.row, bucket.col, bucket.count)?;
        }
        tx.commit()?;
        std::fs::rename(&old, old.with_extension("json.migrated"))?;
    }
    Ok(conn)
}

fn upsert(
    conn: &Connection,
    hour: u64,
    layer: &str,
    row: u8,
    col: u8,
    count: u64,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO key_usage VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (hour, layer, row, col) DO UPDATE SET count = count + excluded.count",
        params![hour, layer, row, col, count],
    )
}

/// Add counts read from the keyboard to the buckets of `hour`.
pub fn add(
    conn: &Connection,
    hour: u64,
    table: Table,
    counts: &[(usize, u16)],
) -> anyhow::Result<()> {
    let tx = conn.unchecked_transaction()?;
    for &(index, count) in counts {
        let count = u64::from(count);
        let (row, col) = (index / COLS % ROWS, index % COLS);
        let (row, col) = (row as u8, col as u8);
        let group = index / (ROWS * COLS);
        match table {
            Table::Keys => {
                upsert(&tx, hour, LAYERS[group], row, col, count)?;
            }
            Table::Mods => {
                tx.execute(
                    "INSERT INTO mod_usage VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (hour, mods, row, col) DO UPDATE SET count = count + excluded.count",
                    params![hour, mods_name(group), row, col, count],
                )?;
            }
            Table::Dual => {
                tx.execute(
                    "INSERT INTO dualrole_usage VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (hour, row, col, kind) DO UPDATE SET count = count + excluded.count",
                    params![hour, row, col, DUAL_KINDS[group], count],
                )?;
            }
            Table::Layers => {
                let (layer, entries, held) = match index {
                    0..5 => (index, count, 0),
                    _ => (index - LAYERS.len(), 0, count * LAYER_TIME_UNIT_MS),
                };
                tx.execute(
                    "INSERT INTO layer_usage VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (hour, layer) DO UPDATE SET
                         entries = entries + excluded.entries, held_ms = held_ms + excluded.held_ms",
                    params![hour, LAYERS[layer], entries, held],
                )?;
            }
        }
    }
    Ok(tx.commit()?)
}

/// Every bucket from `since_hour` on.
pub fn load(
    conn: &Connection,
    since_hour: u64,
) -> anyhow::Result<Vec<Bucket>> {
    let mut stmt =
        conn.prepare("SELECT hour, layer, row, col, count FROM key_usage WHERE hour >= ?1")?;
    let rows = stmt.query_map([since_hour], |r| {
        let name: String = r.get(1)?;
        Ok(Bucket {
            hour: r.get(0)?,
            layer: LAYERS.iter().position(|l| *l == name).unwrap_or(usize::MAX) as u8,
            row: r.get(2)?,
            col: r.get(3)?,
            count: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A text heatmap per layer of the presses since `since_hour`, each key
/// drawn where it sits on the keyboard, then the keys never pressed.
pub fn render(
    buckets: &[Bucket],
    since_hour: u64,
) -> String {
    let mut totals: BTreeMap<(u8, u8, u8), u64> = BTreeMap::new();
    for bucket in buckets.iter().filter(|b| b.hour >= since_hour) {
        *totals
            .entry((bucket.layer, bucket.row, bucket.col))
            .or_default() += bucket.count;
    }
    let mut text = String::new();
    for (layer, name) in LAYERS.iter().enumerate() {
        let total = |row, col| totals.get(&(layer as u8, row, col)).copied().unwrap_or(0);
        text += &format!("{name}\n");
        let lines = KEYS.iter().map(|key| key.2).max().unwrap_or(0);
        let cells = KEYS.iter().map(|key| key.3).max().unwrap_or(0);
        for line in 0..=lines {
            let mut out = String::new();
            for cell in 0..=cells {
                match KEYS.iter().find(|key| (key.2, key.3) == (line, cell)) {
                    Some(&(row, col, ..)) => out += &format!("{:>6}", total(row, col)),
                    None => out += "      ",
                }
            }
            text += out.trim_end();
            text += "\n";
        }
        let never: Vec<String> = KEYS
            .iter()
            .filter(|key| total(key.0, key.1) == 0)
            .map(|key| format!("{},{}", key.0, key.1))
            .collect();
        text += &format!("never pressed (row,col): {}\n\n", never.join(" "));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden vector, shared with tests/protocol_test.c in the keymap.
    #[test]
    fn test_decode() {
        let mut packet = [0; REPORT_LEN];
        packet[..8].copy_from_slice(&[13, 1, 1, 0, 2, 1, 255, 255]);
        // Chunk 1 starts at index 15: layer 0, row 2, col 1 (15 = 2 * 7 + 1).
        assert_eq!(
            decode(&packet),
            Some((Table::Keys, vec![(15, 1), (16, 258), (17, 65535)]))
        );
        packet[0] = 15;
        assert_eq!(decode(&packet).map(|(table, _)| table), Some(Table::Mods));
        assert_eq!(decode(&[0; REPORT_LEN]), None);
    }

    #[test]
    fn test_add_and_render() {
        let dir = std::env::temp_dir().join(format!("key-usage-test-{}", std::process::id()));
        let conn = open(&dir.join("usage.sqlite")).unwrap();
        add(&conn, 10, Table::Keys, &[(0, 3), (3 * 84 + 6 * 7 + 6, 1)]).unwrap();
        add(&conn, 10, Table::Keys, &[(0, 2)]).unwrap();
        add(&conn, 11, Table::Keys, &[(0, 1)]).unwrap();
        let buckets = load(&conn, 0).unwrap();
        assert_eq!(buckets.len(), 3);
        let count = |hour| {
            buckets
                .iter()
                .find(|b| (b.hour, b.layer) == (hour, 0))
                .unwrap()
                .count
        };
        assert_eq!(count(10), 5);
        let layout: i64 = conn
            .query_row("SELECT count(*) FROM key_layout", [], |r| r.get(0))
            .unwrap();
        assert_eq!(layout, 72);
        let labels: i64 = conn
            .query_row("SELECT count(*) FROM key_label", [], |r| r.get(0))
            .unwrap();
        assert_eq!(labels, 288);
        // ctrl+shift (mask 3) on key (1, 2); a tap and a hold of (0, 3); layer entries and time.
        add(&conn, 10, Table::Mods, &[(3 * 84 + 7 + 2, 4)]).unwrap();
        add(&conn, 10, Table::Dual, &[(3, 2), (84 + 3, 1)]).unwrap();
        add(&conn, 10, Table::Layers, &[(1, 3), (5, 25)]).unwrap();
        let text = |sql: &str| -> String { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
        assert_eq!(text("SELECT mods || count FROM mod_usage"), "ctrl+shift4");
        assert_eq!(
            text("SELECT group_concat(kind || count) FROM dualrole_usage"),
            "tap2,hold1"
        );
        assert_eq!(
            text(
                "SELECT layer || entries || '/' || held_ms FROM layer_usage ORDER BY entries DESC LIMIT 1"
            ),
            "symbols3/2500"
        );
        assert_eq!(
            text("SELECT finger || half FROM key_layout WHERE row = 5 AND col = 1"),
            "thumbleft"
        );
        let all = render(&buckets, 0);
        assert!(all.starts_with("qwerty\n     6"));
        assert!(all.contains("media\n"));
        // Dropping hour 10 and older leaves one press.
        assert!(render(&load(&conn, 11).unwrap(), 11).starts_with("qwerty\n     1"));
        assert!(all.contains("never pressed (row,col): 0,1"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_chunks_cover_all_positions() {
        for table in Table::ALL {
            assert!(usize::from(table.chunks()) * PER_CHUNK >= table.total());
        }
        assert_eq!(
            Table::Keys.next(Table::Keys.chunks() - 1),
            Some((Table::Mods, 0))
        );
        assert_eq!(Table::Layers.next(Table::Layers.chunks() - 1), None);
    }
}
