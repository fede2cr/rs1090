//! db — SQLite persistence layer
//!
//! Schema matches CO2_DB_SCHEMA.md. Uses rusqlite (bundled SQLite).

use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;

/// Wrapper around a SQLite connection with CO₂-tracker schema.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the database and apply migrations.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;

        // Performance pragmas for a write-heavy embedded database.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous  = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Create tables and indexes if they don't exist.
    fn migrate(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS flights (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                hex         TEXT    NOT NULL,
                callsign    TEXT,
                type_code   TEXT,
                category    TEXT,
                wtc         TEXT,
                size_class  TEXT,
                country     TEXT,
                country_code TEXT,
                first_seen  TEXT    NOT NULL,
                last_seen   TEXT    NOT NULL,
                date        TEXT    NOT NULL,
                dist_km     REAL    NOT NULL DEFAULT 0,
                co2_kg      REAL    NOT NULL DEFAULT 0,
                ef_used     REAL,
                ef_source   TEXT,
                n_pos       INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS positions (
                hex  TEXT PRIMARY KEY,
                lat  REAL NOT NULL,
                lon  REAL NOT NULL,
                ts   TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_flights_date         ON flights (date);
            CREATE INDEX IF NOT EXISTS idx_flights_hex           ON flights (hex);
            CREATE INDEX IF NOT EXISTS idx_flights_country_code  ON flights (country_code);
            CREATE INDEX IF NOT EXISTS idx_flights_size_class    ON flights (size_class);
            CREATE INDEX IF NOT EXISTS idx_flights_type_code     ON flights (type_code);",
        )?;
        Ok(())
    }

    // ──────────────────────── Position scratch table ────────────────────────

    /// Get last known position for an aircraft hex.
    pub fn get_position(&self, hex: &str) -> SqlResult<Option<(f64, f64, String)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT lat, lon, ts FROM positions WHERE hex = ?1",
        )?;
        let mut rows = stmt.query(params![hex])?;
        match rows.next()? {
            Some(row) => Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?))),
            None => Ok(None),
        }
    }

    /// Upsert the latest position for a hex.
    pub fn set_position(&self, hex: &str, lat: f64, lon: f64, ts: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO positions (hex, lat, lon, ts)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(hex) DO UPDATE SET lat=?2, lon=?3, ts=?4",
            params![hex, lat, lon, ts],
        )?;
        Ok(())
    }

    // ──────────────────────── Flight tracking ──────────────────────────────

    /// Find the most recent open flight row for a hex (last_seen within
    /// `stale_secs` of `now_iso`).
    pub fn find_active_flight(&self, hex: &str, stale_secs: i64) -> SqlResult<Option<i64>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id FROM flights
             WHERE hex = ?1
               AND (julianday('now') - julianday(last_seen)) * 86400 < ?2
             ORDER BY id DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![hex, stale_secs])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// Insert a new flight row, returning the id.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_flight(
        &self,
        hex: &str,
        callsign: Option<&str>,
        type_code: Option<&str>,
        category: Option<&str>,
        wtc: Option<&str>,
        size_class: &str,
        country: Option<&str>,
        country_code: Option<&str>,
        now_iso: &str,
        date: &str,
        ef_used: f64,
        ef_source: &str,
    ) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO flights
                (hex, callsign, type_code, category, wtc, size_class,
                 country, country_code, first_seen, last_seen, date,
                 ef_used, ef_source, n_pos)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,?10,?11,?12,1)",
            params![
                hex, callsign, type_code, category, wtc, size_class,
                country, country_code, now_iso, date, ef_used, ef_source,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Accumulate distance/CO₂ and update last_seen on an existing flight.
    pub fn update_flight(
        &self,
        id: i64,
        delta_km: f64,
        delta_co2: f64,
        now_iso: &str,
    ) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE flights
             SET dist_km  = dist_km + ?1,
                 co2_kg   = co2_kg  + ?2,
                 last_seen = ?3,
                 n_pos     = n_pos + 1
             WHERE id = ?4",
            params![delta_km, delta_co2, now_iso, id],
        )?;
        Ok(())
    }

    // ──────────────────────── Query helpers (JSON API) ──────────────────────

    /// All-time totals.
    pub fn totals(&self) -> SqlResult<(f64, f64, i64)> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(co2_kg),0), COALESCE(SUM(dist_km),0), COUNT(*)
             FROM flights",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
    }

    /// Today's totals.
    pub fn today_totals(&self) -> SqlResult<(f64, f64, i64)> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(co2_kg),0), COALESCE(SUM(dist_km),0), COUNT(*)
             FROM flights WHERE date = date('now')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
    }

    /// Daily history (last N days).
    pub fn daily_history(&self, days: u32) -> SqlResult<Vec<DayRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, SUM(co2_kg), SUM(dist_km), COUNT(*)
             FROM flights
             WHERE date >= date('now', ?1)
             GROUP BY date ORDER BY date",
        )?;
        let offset = format!("-{days} days");
        let rows = stmt.query_map(params![offset], |row| {
            Ok(DayRow {
                date: row.get(0)?,
                co2_kg: row.get(1)?,
                dist_km: row.get(2)?,
                flights: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Today's CO₂ broken down by size_class.
    pub fn today_by_size(&self) -> SqlResult<Vec<(String, f64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT size_class, COALESCE(SUM(co2_kg),0), COUNT(*)
             FROM flights WHERE date = date('now')
             GROUP BY size_class",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect()
    }

    /// Today's top countries by CO₂.
    pub fn today_top_countries(&self, limit: u32) -> SqlResult<Vec<(String, f64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(country_code,'??'), COALESCE(SUM(co2_kg),0), COUNT(*)
             FROM flights WHERE date = date('now')
             GROUP BY country_code
             ORDER BY SUM(co2_kg) DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect()
    }

    /// Get direct reference to the connection (for transactions etc.)
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// A row from the daily history query.
#[derive(Debug)]
pub struct DayRow {
    pub date: String,
    pub co2_kg: f64,
    pub dist_km: f64,
    pub flights: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> Database {
        Database::open(Path::new(":memory:")).expect("in-memory db")
    }

    #[test]
    fn schema_creates() {
        let _db = mem_db();
    }

    #[test]
    fn position_roundtrip() {
        let db = mem_db();
        db.set_position("abc123", 50.0, 8.0, "2026-02-26T12:00:00Z").unwrap();
        let (lat, lon, _ts) = db.get_position("abc123").unwrap().unwrap();
        assert!((lat - 50.0).abs() < 1e-9);
        assert!((lon - 8.0).abs() < 1e-9);
    }

    #[test]
    fn flight_insert_and_update() {
        let db = mem_db();
        let id = db.insert_flight(
            "abc123", Some("DLH1A"), Some("A320"), Some("A3"), None,
            "medium", Some("Germany"), Some("de"),
            "2026-02-26T12:00:00Z", "2026-02-26",
            9.5, "type",
        ).unwrap();
        assert!(id > 0);

        db.update_flight(id, 10.0, 95.0, "2026-02-26T12:05:00Z").unwrap();

        let (co2, dist, count) = db.totals().unwrap();
        assert!((co2 - 95.0).abs() < 1e-6);
        assert!((dist - 10.0).abs() < 1e-6);
        assert_eq!(count, 1);
    }
}
