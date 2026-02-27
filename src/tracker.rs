//! tracker — aircraft state machine
//!
//! Merges individual Mode S / ADS-B messages into a per-aircraft state
//! object. Handles CPR position decoding (pairing odd+even frames),
//! callsign, altitude, velocity, squawk, category, and timeouts.
//!
//! Each aircraft is identified by its 24-bit ICAO address.

use adsb_deku::adsb::{
    AirborneVelocitySubType, Identification, ME, OperationStatus,
};
use adsb_deku::deku::DekuContainerRead;
use adsb_deku::{Altitude, CPRFormat, DF, Frame, ICAO};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, trace};

use crate::co2;
use crate::icao_country;

/// How long before an aircraft with no messages is removed (seconds).
const AIRCRAFT_TIMEOUT_SECS: u64 = 300;

/// Maximum age for a CPR frame to be usable for global decode (seconds).
const CPR_MAX_AGE_SECS: u64 = 10;

/// Per-aircraft tracked state.
#[derive(Debug, Clone)]
pub struct AircraftState {
    pub hex: String,
    pub icao_raw: u32,

    // Identification
    pub callsign: Option<String>,
    pub category: Option<String>, // e.g. "A3", "B2"
    pub registration: Option<String>,
    pub type_code: Option<String>, // from external DB, not ADS-B

    // Position
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub nic: Option<u8>,

    // Altitude
    pub alt_baro: Option<AltitudeValue>,
    pub alt_geom: Option<i32>,

    // Velocity
    pub gs: Option<f64>,         // ground speed, knots
    pub track: Option<f64>,      // degrees true
    pub baro_rate: Option<i16>,  // ft/min
    pub geom_rate: Option<i16>,  // ft/min
    pub ias: Option<f64>,        // indicated airspeed
    pub tas: Option<f64>,        // true airspeed
    pub mach: Option<f64>,

    // Transponder
    pub squawk: Option<String>,
    pub emergency: Option<String>,
    pub version: Option<u8>,       // ADS-B version (0, 1, 2)
    pub nac_p: Option<u8>,
    pub nac_v: Option<u8>,
    pub sil: Option<u8>,
    pub sil_type: Option<String>,

    // Signal / message stats
    pub rssi: Option<f64>,
    pub messages: u64,

    // Timing
    pub seen: f64,          // seconds since last message
    pub seen_pos: f64,      // seconds since last position
    pub last_message: Instant,
    pub last_position: Option<Instant>,

    // Country (from ICAO hex range)
    pub country: Option<String>,
    pub country_code: Option<String>,

    // CPR state (internal, not serialised)
    cpr_even: Option<CprFrame>,
    cpr_odd: Option<CprFrame>,

    // CO₂ tracking
    pub prev_lat: Option<f64>,
    pub prev_lon: Option<f64>,
    pub dist_km: f64,
    pub co2_kg: f64,
    pub ef_used: Option<f64>,
    pub ef_source: Option<co2::EfSource>,
}

/// Barometric altitude: either a numeric value or "ground".
#[derive(Debug, Clone, Copy)]
pub enum AltitudeValue {
    Feet(i32),
    Ground,
}

/// Saved CPR frame for position decoding.
#[derive(Debug, Clone)]
struct CprFrame {
    altitude: Altitude,
    received: Instant,
}

/// The aircraft tracker / state manager.
pub struct Tracker {
    pub aircraft: HashMap<String, AircraftState>,
    pub total_messages: u64,
    receiver_lat: Option<f64>,
    receiver_lon: Option<f64>,
}

impl Tracker {
    pub fn new(receiver_lat: Option<f64>, receiver_lon: Option<f64>) -> Self {
        Self {
            aircraft: HashMap::new(),
            total_messages: 0,
            receiver_lat,
            receiver_lon,
        }
    }

    /// Process a decoded adsb_deku Frame and update aircraft state.
    pub fn update(&mut self, frame: &Frame, signal: Option<u8>) {
        self.total_messages += 1;

        match &frame.df {
            DF::ADSB(adsb) => {
                let icao = icao_to_u32(&adsb.icao);
                let hex = format!("{:06x}", icao);
                let ac = self.get_or_create(&hex, icao);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;

                if let Some(sig) = signal {
                    // Convert to dBFS: 10 * log10(signal/255) approximately
                    if sig > 0 {
                        ac.rssi = Some(10.0 * (f64::from(sig) / 255.0).log10());
                    }
                }

                Self::apply_me(&mut *ac, &adsb.me);
            }

            DF::TisB { cf, .. } => {
                if let Some(me) = extract_tisb_me(cf) {
                    let icao = frame.crc;
                    let hex = format!("~{:06x}", icao); // non-ICAO prefix
                    let ac = self.get_or_create(&hex, icao);
                    ac.messages += 1;
                    ac.last_message = Instant::now();
                    ac.seen = 0.0;
                    Self::apply_me(&mut *ac, me);
                }
            }

            DF::AllCallReply { icao, .. } => {
                let hex = format!("{icao}");
                let icao_val = icao_to_u32(icao);
                let ac = self.get_or_create(&hex, icao_val);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;
            }

            DF::SurveillanceAltitudeReply { ac: alt, .. } => {
                let icao = frame.crc;
                let hex = format!("{:06x}", icao);
                let ac = self.get_or_create(&hex, icao);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;
                if alt.0 > 0 {
                    ac.alt_baro = Some(AltitudeValue::Feet(alt.0 as i32));
                }
            }

            DF::SurveillanceIdentityReply { id, .. } => {
                let icao = frame.crc;
                let hex = format!("{:06x}", icao);
                let ac = self.get_or_create(&hex, icao);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;
                ac.squawk = Some(format!("{:04x}", id.0));
            }

            DF::ShortAirAirSurveillance { altitude, .. } => {
                let icao = frame.crc;
                let hex = format!("{:06x}", icao);
                let ac = self.get_or_create(&hex, icao);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;
                if altitude.0 > 0 {
                    ac.alt_baro = Some(AltitudeValue::Feet(altitude.0 as i32));
                }
            }

            DF::LongAirAir { altitude, .. } => {
                let icao = frame.crc;
                let hex = format!("{:06x}", icao);
                let ac = self.get_or_create(&hex, icao);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;
                if altitude.0 > 0 {
                    ac.alt_baro = Some(AltitudeValue::Feet(altitude.0 as i32));
                }
            }

            DF::CommBAltitudeReply { alt, .. } => {
                let icao = frame.crc;
                let hex = format!("{:06x}", icao);
                let ac = self.get_or_create(&hex, icao);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;
                if alt.0 > 0 {
                    ac.alt_baro = Some(AltitudeValue::Feet(alt.0 as i32));
                }
            }

            DF::CommBIdentityReply { id, .. } => {
                let icao = frame.crc;
                let hex = format!("{:06x}", icao);
                let ac = self.get_or_create(&hex, icao);
                ac.messages += 1;
                ac.last_message = Instant::now();
                ac.seen = 0.0;
                ac.squawk = Some(format!("{:04x}", id));
            }

            _ => {}
        }
    }

    /// Apply an ADS-B ME (Message Extended) to an aircraft state.
    fn apply_me(ac: &mut AircraftState, me: &ME) {
        match me {
            ME::AircraftIdentification(Identification { tc, ca, cn }) => {
                ac.callsign = Some(cn.to_string().trim().to_string());
                ac.category = Some(format!("{tc}{ca}"));
                trace!(hex = %ac.hex, callsign = ?ac.callsign, "identification");
            }

            ME::AirbornePositionBaroAltitude(altitude) => {
                match altitude.alt {
                    Some(alt) if alt > 0 => {
                        ac.alt_baro = Some(AltitudeValue::Feet(alt as i32));
                    }
                    _ => {
                        ac.alt_baro = Some(AltitudeValue::Ground);
                    }
                }
                ac.store_cpr(altitude.clone());
                ac.try_decode_position();
            }

            ME::AirbornePositionGNSSAltitude(altitude) => {
                if let Some(alt) = altitude.alt {
                    if alt > 0 {
                        ac.alt_geom = Some(alt as i32);
                    }
                }
                ac.store_cpr(altitude.clone());
                ac.try_decode_position();
            }

            ME::SurfacePosition(_surface) => {
                ac.alt_baro = Some(AltitudeValue::Ground);
                // Surface positions also use CPR but we skip for now
            }

            ME::AirborneVelocity(av) => {
                if let Some((heading, speed, vrate)) = av.calculate() {
                    ac.gs = Some(speed);
                    ac.track = Some(heading as f64);
                    ac.baro_rate = Some(vrate);
                }

                match &av.sub_type {
                    AirborneVelocitySubType::AirspeedDecoding(airspeed) => {
                        if airspeed.airspeed > 0 {
                            if airspeed.airspeed_type != 0 {
                                ac.tas = Some(airspeed.airspeed as f64);
                            } else {
                                ac.ias = Some(airspeed.airspeed as f64);
                            }
                        }
                    }
                    _ => {}
                }
            }

            ME::AircraftStatus(_status) => {
                trace!(hex = %ac.hex, "aircraft status received");
            }

            ME::TargetStateAndStatusInformation(tssi) => {
                ac.nac_p = Some(tssi.nacp);
                ac.sil = Some(tssi.sil);
            }

            ME::AircraftOperationStatus(op) => {
                match op {
                    OperationStatus::Airborne(a) => {
                        ac.version = Some(adsb_version_to_u8(&a.version_number));
                        ac.nac_p = Some(a.navigational_accuracy_category);
                        ac.sil = Some(a.source_integrity_level);
                    }
                    OperationStatus::Surface(s) => {
                        ac.version = Some(adsb_version_to_u8(&s.version_number));
                        ac.nac_p = Some(s.navigational_accuracy_category);
                        ac.sil = Some(s.source_integrity_level);
                    }
                    _ => {}
                }
            }

            _ => {}
        }
    }

    /// Get existing aircraft or create a new entry.
    fn get_or_create(&mut self, hex: &str, icao: u32) -> &mut AircraftState {
        self.aircraft
            .entry(hex.to_string())
            .or_insert_with(|| AircraftState::new(hex, icao))
    }

    /// Remove aircraft that haven't been seen recently.
    pub fn expire_stale(&mut self) {
        let now = Instant::now();
        self.aircraft.retain(|_hex, ac| {
            now.duration_since(ac.last_message).as_secs() < AIRCRAFT_TIMEOUT_SECS
        });
    }

    /// Update `seen` / `seen_pos` seconds for all aircraft (call before serialising).
    pub fn update_ages(&mut self) {
        let now = Instant::now();
        for ac in self.aircraft.values_mut() {
            ac.seen = now.duration_since(ac.last_message).as_secs_f64();
            if let Some(last_pos) = ac.last_position {
                ac.seen_pos = now.duration_since(last_pos).as_secs_f64();
            }
        }
    }

    /// Number of aircraft currently tracked.
    pub fn count(&self) -> usize {
        self.aircraft.len()
    }

    /// Number of aircraft with a valid position.
    pub fn count_with_position(&self) -> usize {
        self.aircraft
            .values()
            .filter(|ac| ac.lat.is_some())
            .count()
    }
}

impl AircraftState {
    fn new(hex: &str, icao: u32) -> Self {
        let country_info = icao_country::lookup(hex.trim_start_matches('~'));
        let (ef_val, ef_src) = co2::emission_factor(None, None, None);

        Self {
            hex: hex.to_string(),
            icao_raw: icao,
            callsign: None,
            category: None,
            registration: None,
            type_code: None,
            lat: None,
            lon: None,
            nic: None,
            alt_baro: None,
            alt_geom: None,
            gs: None,
            track: None,
            baro_rate: None,
            geom_rate: None,
            ias: None,
            tas: None,
            mach: None,
            squawk: None,
            emergency: None,
            version: None,
            nac_p: None,
            nac_v: None,
            sil: None,
            sil_type: None,
            rssi: None,
            messages: 0,
            seen: 0.0,
            seen_pos: f64::MAX,
            last_message: Instant::now(),
            last_position: None,
            country: country_info.as_ref().map(|c| c.country.to_string()),
            country_code: country_info.as_ref().map(|c| c.code.to_string()),
            cpr_even: None,
            cpr_odd: None,
            prev_lat: None,
            prev_lon: None,
            dist_km: 0.0,
            co2_kg: 0.0,
            ef_used: Some(ef_val),
            ef_source: Some(ef_src),
        }
    }

    /// Store a CPR frame (even or odd) for deferred position decoding.
    fn store_cpr(&mut self, altitude: Altitude) {
        let frame = CprFrame {
            altitude: altitude.clone(),
            received: Instant::now(),
        };
        match altitude.odd_flag {
            CPRFormat::Even => self.cpr_even = Some(frame),
            CPRFormat::Odd => self.cpr_odd = Some(frame),
        }
    }

    /// Attempt CPR position decode using stored even+odd frames.
    fn try_decode_position(&mut self) {
        let (even, odd) = match (&self.cpr_even, &self.cpr_odd) {
            (Some(e), Some(o)) => (e, o),
            _ => return,
        };

        // Check both frames are recent enough
        let now = Instant::now();
        if now.duration_since(even.received).as_secs() > CPR_MAX_AGE_SECS
            || now.duration_since(odd.received).as_secs() > CPR_MAX_AGE_SECS
        {
            return;
        }

        // Global CPR decode
        if let Some(pos) =
            adsb_deku::cpr::get_position((&even.altitude, &odd.altitude))
        {
            // Sanity check: latitude ±90, longitude ±180
            if pos.latitude.abs() <= 90.0 && pos.longitude.abs() <= 180.0 {
                self.update_position(pos.latitude, pos.longitude);
            }
        }
    }

    /// Update position and compute CO₂ distance delta.
    fn update_position(&mut self, lat: f64, lon: f64) {
        // CO₂ distance tracking
        if let (Some(prev_lat), Some(prev_lon)) = (self.prev_lat, self.prev_lon) {
            let dist = co2::haversine_km(prev_lat, prev_lon, lat, lon);
            if dist > co2::MIN_MOVE_KM && dist < co2::MAX_JUMP_KM {
                self.dist_km += dist;

                // Update emission factor if we now know the type
                let (ef, src) = co2::emission_factor(
                    self.type_code.as_deref(),
                    None, // WTC from DB not available yet
                    self.category.as_deref(),
                );
                self.ef_used = Some(ef);
                self.ef_source = Some(src);
                self.co2_kg += dist * ef;
            }
        }

        self.prev_lat = Some(lat);
        self.prev_lon = Some(lon);
        self.lat = Some(lat);
        self.lon = Some(lon);
        self.last_position = Some(Instant::now());
        self.seen_pos = 0.0;

        debug!(hex = %self.hex, lat, lon, "position update");
    }
}

/// Convert an adsb_deku ICAO to u32.
fn icao_to_u32(icao: &ICAO) -> u32 {
    u32::from(icao.0[0]) << 16 | u32::from(icao.0[1]) << 8 | u32::from(icao.0[2])
}

/// Convert ADSBVersion enum to u8.
fn adsb_version_to_u8(v: &adsb_deku::adsb::ADSBVersion) -> u8 {
    use adsb_deku::adsb::ADSBVersion;
    match v {
        ADSBVersion::DOC9871AppendixA => 0,
        ADSBVersion::DOC9871AppendixB => 1,
        ADSBVersion::DOC9871AppendixC => 2,
    }
}

/// Extract ME from TIS-B ControlField.
fn extract_tisb_me(cf: &adsb_deku::adsb::ControlField) -> Option<&ME> {
    Some(&cf.me)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_frame(bytes: &[u8]) -> Option<Frame> {
        Frame::from_bytes((bytes, 0)).ok().map(|(_, f)| f)
    }

    #[test]
    fn tracker_creates_aircraft() {
        let mut tracker = Tracker::new(None, None);
        assert_eq!(tracker.count(), 0);

        let bytes = hex_to_bytes("8da2c1bd587ba2adb31799cb802b");
        if let Some(frame) = parse_frame(&bytes) {
            tracker.update(&frame, Some(0x80));
            assert_eq!(tracker.count(), 1);
            let ac = tracker.aircraft.values().next().unwrap();
            assert_eq!(ac.hex, "a2c1bd");
            assert!(ac.messages > 0);
        }
    }

    #[test]
    fn tracker_decodes_identity() {
        let mut tracker = Tracker::new(None, None);

        let bytes = hex_to_bytes("8da61789200464b3cf6c207c7b06");
        if let Some(frame) = parse_frame(&bytes) {
            tracker.update(&frame, None);
            let ac = tracker.aircraft.get("a61789").unwrap();
            assert!(ac.callsign.is_some(), "callsign should be decoded");
        }
    }

    #[test]
    fn tracker_expires_stale() {
        let mut tracker = Tracker::new(None, None);

        let bytes = hex_to_bytes("8da2c1bd587ba2adb31799cb802b");
        if let Some(frame) = parse_frame(&bytes) {
            tracker.update(&frame, None);
            assert_eq!(tracker.count(), 1);

            let ac = tracker.aircraft.values_mut().next().unwrap();
            ac.last_message = Instant::now() - std::time::Duration::from_secs(AIRCRAFT_TIMEOUT_SECS + 1);

            tracker.expire_stale();
            assert_eq!(tracker.count(), 0);
        }
    }

    #[test]
    fn country_lookup_on_create() {
        let mut tracker = Tracker::new(None, None);

        let bytes = hex_to_bytes("8d3c6752587ba2adb31799cb802b");
        if let Some(frame) = parse_frame(&bytes) {
            tracker.update(&frame, None);
            let ac = tracker.aircraft.get("3c6752").unwrap();
            assert_eq!(ac.country_code.as_deref(), Some("de"));
        }
    }

    fn hex_to_bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }
}
