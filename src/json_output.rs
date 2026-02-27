//! json_output — write tar1090-compatible JSON files
//!
//! Generates `receiver.json` (once) and `aircraft.json` (every second)
//! in the format that tar1090's web UI expects, matching readsb's output.
//!
//! Files are written atomically (write to `.tmp`, then rename).

use serde::Serialize;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

use crate::tracker::{AircraftState, AltitudeValue, Tracker};

/// receiver.json — written once at startup.
#[derive(Debug, Serialize)]
pub struct ReceiverJson {
    pub version: String,
    pub refresh: u64,
    pub history: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// aircraft.json — written every cycle.
#[derive(Debug, Serialize)]
pub struct AircraftJsonOutput {
    pub now: f64,
    pub messages: u64,
    pub aircraft: Vec<AircraftEntry>,
}

/// A single aircraft entry in aircraft.json (tar1090 compatible).
#[derive(Debug, Serialize)]
pub struct AircraftEntry {
    pub hex: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub flight: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_baro: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_geom: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gs: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub baro_rate: Option<i16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geom_rate: Option<i16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub squawk: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nic: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seen_pos: Option<f64>,

    pub seen: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi: Option<f64>,

    pub messages: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ias: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tas: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mach: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nac_p: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nac_v: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sil: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sil_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency: Option<String>,

    // Database-enriched fields (from tar1090-db)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<String>, // registration

    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>, // ICAO type code

    #[serde(rename = "dbFlags", skip_serializing_if = "Option::is_none")]
    pub db_flags: Option<u32>,
}

/// Write `receiver.json` to the output directory.
pub fn write_receiver_json(
    output_dir: &Path,
    receiver_lat: Option<f64>,
    receiver_lon: Option<f64>,
) -> anyhow::Result<()> {
    let receiver = ReceiverJson {
        version: format!("rs1090 {}", env!("CARGO_PKG_VERSION")),
        refresh: 1000, // ms
        history: 0,    // no history files yet
        lat: receiver_lat,
        lon: receiver_lon,
    };

    let path = output_dir.join("receiver.json");
    let tmp = output_dir.join("receiver.json.tmp");
    let data = serde_json::to_string(&receiver)?;
    std::fs::write(&tmp, &data)?;
    std::fs::rename(&tmp, &path)?;
    debug!(?path, "wrote receiver.json");
    Ok(())
}

/// Write `aircraft.json` from the current tracker state.
pub fn write_aircraft_json(
    output_dir: &Path,
    tracker: &Tracker,
) -> anyhow::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    let aircraft: Vec<AircraftEntry> = tracker
        .aircraft
        .values()
        .map(|ac| aircraft_to_entry(ac))
        .collect();

    let output = AircraftJsonOutput {
        now,
        messages: tracker.total_messages,
        aircraft,
    };

    let path = output_dir.join("aircraft.json");
    let tmp = output_dir.join("aircraft.json.tmp");
    let data = serde_json::to_string(&output)?;
    std::fs::write(&tmp, &data)?;
    std::fs::rename(&tmp, &path)?;

    Ok(())
}

/// Convert internal AircraftState to the tar1090-compatible JSON entry.
fn aircraft_to_entry(ac: &AircraftState) -> AircraftEntry {
    let alt_baro = ac.alt_baro.map(|a| match a {
        AltitudeValue::Feet(ft) => serde_json::Value::Number(ft.into()),
        AltitudeValue::Ground => serde_json::Value::String("ground".to_string()),
    });

    // Pad callsign to 8 chars (tar1090 expects this)
    let flight = ac.callsign.as_ref().map(|cs| format!("{:<8}", cs));

    let seen_pos = if ac.lat.is_some() {
        Some(ac.seen_pos)
    } else {
        None
    };

    AircraftEntry {
        hex: ac.hex.clone(),
        flight,
        alt_baro,
        alt_geom: ac.alt_geom,
        gs: ac.gs.map(|v| (v * 10.0).round() / 10.0), // 1 decimal
        track: ac.track.map(|v| (v * 10.0).round() / 10.0),
        baro_rate: ac.baro_rate,
        geom_rate: ac.geom_rate,
        squawk: ac.squawk.clone(),
        category: ac.category.clone(),
        lat: ac.lat.map(|v| (v * 1_000_000.0).round() / 1_000_000.0),
        lon: ac.lon.map(|v| (v * 1_000_000.0).round() / 1_000_000.0),
        nic: ac.nic,
        seen_pos,
        seen: (ac.seen * 10.0).round() / 10.0,
        rssi: ac.rssi.map(|v| (v * 10.0).round() / 10.0),
        messages: ac.messages,
        ias: ac.ias,
        tas: ac.tas,
        mach: ac.mach,
        version: ac.version,
        nac_p: ac.nac_p,
        nac_v: ac.nac_v,
        sil: ac.sil,
        sil_type: ac.sil_type.clone(),
        emergency: ac.emergency.clone(),
        r: ac.registration.clone(),
        t: ac.type_code.clone(),
        db_flags: None, // TODO: from tar1090-db
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_json_roundtrip() {
        let r = ReceiverJson {
            version: "rs1090 0.1.0".into(),
            refresh: 1000,
            history: 0,
            lat: Some(48.12),
            lon: Some(11.56),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("rs1090"));
        assert!(json.contains("1000"));
        assert!(json.contains("48.12"));
    }

    #[test]
    fn altitude_ground_serialization() {
        let entry = AircraftEntry {
            hex: "abc123".into(),
            flight: None,
            alt_baro: Some(serde_json::Value::String("ground".into())),
            alt_geom: None,
            gs: None,
            track: None,
            baro_rate: None,
            geom_rate: None,
            squawk: None,
            category: None,
            lat: Some(50.0),
            lon: Some(8.0),
            nic: None,
            seen_pos: Some(0.0),
            seen: 0.5,
            rssi: None,
            messages: 10,
            ias: None,
            tas: None,
            mach: None,
            version: None,
            nac_p: None,
            nac_v: None,
            sil: None,
            sil_type: None,
            emergency: None,
            r: None,
            t: None,
            db_flags: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""alt_baro":"ground""#));
    }

    #[test]
    fn optional_fields_omitted() {
        let entry = AircraftEntry {
            hex: "abc123".into(),
            flight: None,
            alt_baro: None,
            alt_geom: None,
            gs: None,
            track: None,
            baro_rate: None,
            geom_rate: None,
            squawk: None,
            category: None,
            lat: None,
            lon: None,
            nic: None,
            seen_pos: None,
            seen: 1.0,
            rssi: None,
            messages: 1,
            ias: None,
            tas: None,
            mach: None,
            version: None,
            nac_p: None,
            nac_v: None,
            sil: None,
            sil_type: None,
            emergency: None,
            r: None,
            t: None,
            db_flags: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        // Optional fields should not appear
        assert!(!json.contains("alt_baro"));
        assert!(!json.contains("flight"));
        assert!(!json.contains("squawk"));
        // Required fields should
        assert!(json.contains("hex"));
        assert!(json.contains("seen"));
        assert!(json.contains("messages"));
    }
}
