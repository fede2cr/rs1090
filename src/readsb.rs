//! readsb — parse aircraft.json from readsb
//!
//! The JSON format is documented at:
//! <https://github.com/wiedehopf/readsb/blob/dev/README-json.md>

use serde::Deserialize;
use std::path::Path;

/// Top-level structure of aircraft.json
#[derive(Debug, Deserialize)]
pub struct AircraftJson {
    /// Unix epoch of the file snapshot
    pub now: f64,
    /// Number of entries with position
    #[serde(default)]
    pub messages: u64,
    /// Individual aircraft
    #[serde(default)]
    pub aircraft: Vec<Aircraft>,
}

/// A single aircraft entry from readsb.
///
/// Only fields relevant to CO₂ tracking are modelled; the rest are
/// silently ignored via `#[serde(deny_unknown_fields)]` being absent.
#[derive(Debug, Deserialize)]
pub struct Aircraft {
    /// ICAO 24-bit hex address (lowercase string, e.g. "3c6752")
    pub hex: String,

    /// Aircraft type ICAO designator (e.g. "A320", "B738")
    /// Populated from the local database (tar1090-db).
    #[serde(rename = "t")]
    pub type_code: Option<String>,

    /// Flight callsign (may have trailing spaces)
    pub flight: Option<String>,

    /// ADS-B emitter category string (e.g. "A3", "B2")
    pub category: Option<String>,

    /// Latitude in degrees
    pub lat: Option<f64>,

    /// Longitude in degrees
    pub lon: Option<f64>,

    /// Seconds since last position update
    pub seen_pos: Option<f64>,

    /// Seconds since any message from this aircraft
    pub seen: Option<f64>,

    /// Wake turbulence category descriptor from the database
    /// Injected by readsb when db info is available.
    #[serde(rename = "r")]
    pub registration: Option<String>,

    /// Database flags field — contains WTC among other things
    #[serde(rename = "dbFlags")]
    pub db_flags: Option<u32>,
}

impl Aircraft {
    /// Whether this aircraft has a recent, valid position.
    pub fn has_valid_position(&self, stale_secs: f64) -> bool {
        self.lat.is_some()
            && self.lon.is_some()
            && self.seen_pos.map_or(false, |s| s < stale_secs)
    }

    /// Trimmed callsign, if present.
    pub fn callsign(&self) -> Option<&str> {
        self.flight.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty())
    }
}

/// Read and parse aircraft.json from disk.
pub fn read_aircraft_json(path: &Path) -> anyhow::Result<AircraftJson> {
    let data = std::fs::read(path)?;
    let parsed: AircraftJson = serde_json::from_slice(&data)?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal() {
        let json = r#"{"now":1700000000,"aircraft":[
            {"hex":"3c6752","t":"A320","flight":"DLH1A  ","category":"A3","lat":50.0,"lon":8.0,"seen_pos":1.2,"seen":0.5}
        ]}"#;
        let parsed: AircraftJson = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.aircraft.len(), 1);
        let ac = &parsed.aircraft[0];
        assert_eq!(ac.hex, "3c6752");
        assert_eq!(ac.type_code.as_deref(), Some("A320"));
        assert_eq!(ac.callsign(), Some("DLH1A"));
        assert!(ac.has_valid_position(120.0));
    }

    #[test]
    fn stale_position_rejected() {
        let json = r#"{"now":1700000000,"aircraft":[
            {"hex":"aaaaaa","lat":40.0,"lon":-74.0,"seen_pos":200.0,"seen":0.1}
        ]}"#;
        let parsed: AircraftJson = serde_json::from_str(json).unwrap();
        assert!(!parsed.aircraft[0].has_valid_position(120.0));
    }
}
