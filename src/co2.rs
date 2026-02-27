//! co2 — emission factor lookup and distance computation
//!
//! All factors in **kg CO₂ per km** (total aircraft, combustion only).
//! Derived as fuel-burn (kg/km) × 3.16 (IPCC kerosene factor).
//! See CO2_METHODOLOGY.md for full sourcing.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Haversine distance in km between two (lat, lon) points.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371.0; // mean Earth radius, km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().atan2((1.0 - a).sqrt())
}

/// Maximum jump distance — larger gaps are treated as lost tracking.
pub const MAX_JUMP_KM: f64 = 50.0;
/// Minimum movement — below this is GPS jitter.
pub const MIN_MOVE_KM: f64 = 0.01;

// ──────────────────────────── Emission factors ────────────────────────────

/// Per-ICAO-type emission factors (kg CO₂/km, total aircraft).
static EF_TYPE: LazyLock<HashMap<&'static str, f64>> = LazyLock::new(|| {
    HashMap::from([
        // ── Turboprops ───────────────────────────────────────────
        ("AT43", 4.1), ("AT45", 4.0), ("AT72", 4.9), ("AT76", 4.7),
        ("DH8A", 4.4), ("DH8B", 4.7), ("DH8C", 5.4), ("DH8D", 6.5),
        ("SF34", 3.2), ("D328", 3.6), ("F50",  4.7), ("JS41", 3.2),
        ("L410", 2.1), ("AN26", 7.9), ("AN24", 7.0), ("BEH2", 3.2),
        // ── Regional jets ────────────────────────────────────────
        ("CRJ1", 5.9), ("CRJ2", 5.7), ("CRJ7", 7.7), ("CRJ9", 8.8), ("CRJX", 8.4),
        ("E135", 4.6), ("E145", 4.9), ("E170", 8.2), ("E75L", 8.8), ("E75S", 8.8),
        ("E190", 10.2), ("E195", 10.1), ("E290", 7.8), ("E295", 8.3),
        ("F70",  7.3), ("F100", 8.8), ("RJ85", 9.5), ("RJ1H", 10.1),
        ("SU95", 8.9), ("AR85", 8.8),
        // ── Narrow-body ──────────────────────────────────────────
        ("A318", 8.5), ("A319", 9.3), ("A19N", 7.6),
        ("A320", 9.5), ("A20N", 8.8),
        ("A321", 11.4), ("A21N", 10.7),
        ("B731", 9.5), ("B732", 10.1), ("B733", 10.1),
        ("B734", 10.4), ("B735", 9.5), ("B736", 8.8),
        ("B737", 8.9), ("B738", 10.0), ("B739", 10.8),
        ("B37M", 7.9), ("B38M", 8.6), ("B39M", 9.2),
        ("B752", 13.9), ("B753", 14.8),
        ("MD80", 11.1), ("MD81", 10.7), ("MD82", 11.1), ("MD83", 11.1),
        ("MD87", 10.1), ("MD88", 11.1), ("MD90", 10.4),
        ("BCS1", 7.2), ("BCS3", 7.7),
        ("C919", 9.8), ("B712", 8.8),
        ("DC93", 8.8), ("DC95", 9.5),
        ("T204", 10.4), ("T154", 17.4),
        // ── Wide-body ────────────────────────────────────────────
        ("A306", 20.5), ("A30B", 22.1), ("A310", 17.4),
        ("A332", 19.6), ("A333", 20.6),
        ("A338", 17.2), ("A339", 18.9),
        ("A342", 22.1), ("A343", 22.3), ("A345", 25.3), ("A346", 26.9),
        ("A359", 20.7), ("A35K", 23.8), ("A388", 43.5),
        ("B741", 37.9), ("B742", 37.9), ("B743", 37.9),
        ("B744", 36.5), ("B748", 33.0),
        ("B762", 15.5), ("B763", 17.2), ("B764", 18.5),
        ("B772", 21.6), ("B77L", 23.9), ("B77W", 27.4),
        ("B788", 16.8), ("B789", 18.1), ("B78X", 19.5),
        ("DC10", 26.9), ("MD11", 26.9),
        ("L101", 26.9), ("IL96", 28.4), ("IL86", 31.6),
        // ── Business / private jets ──────────────────────────────
        ("C25A", 1.2), ("C25B", 1.3), ("C25C", 1.5), ("C25M", 1.6),
        ("C510", 1.0), ("C525", 1.2),
        ("C500", 1.1), ("C550", 1.4), ("C560", 1.8), ("C56X", 2.0),
        ("C680", 2.4), ("C68A", 2.4), ("C700", 2.6), ("C750", 3.5),
        ("CL30", 2.9), ("CL35", 2.9), ("CL60", 3.2),
        ("GL5T", 5.1), ("GL7T", 5.5), ("GLEX", 5.3),
        ("GLF4", 3.5), ("GLF5", 5.9), ("GLF6", 5.4),
        ("G150", 1.8), ("G280", 2.4),
        ("FA50", 2.2), ("FA7X", 3.4), ("FA8X", 3.5), ("F900", 2.7), ("F2TH", 2.4),
        ("E35L", 1.6), ("E55P", 1.6),
        ("LJ35", 2.0), ("LJ45", 2.1), ("LJ60", 2.4), ("LJ75", 2.3),
        ("H25B", 2.5), ("H25C", 3.0),
        ("GALX", 2.6), ("ASTR", 1.8),
        ("PC12", 1.4), ("PC24", 1.6), ("TBM7", 1.1), ("TBM8", 1.1), ("TBM9", 1.1),
        ("PRM1", 1.5), ("P180", 1.4),
        ("BE20", 1.4), ("BE30", 1.9), ("BE40", 1.6), ("BE4W", 1.7),
        ("EA50", 1.0),
        // ── Military transport / tanker ──────────────────────────
        ("C130", 14.2), ("C30J", 12.6), ("C17",  31.6), ("C5",   45.8), ("C5M", 45.8),
        ("K35R", 26.9), ("KC10", 26.9), ("A400", 15.2), ("MRTT", 20.5),
        ("A124", 56.9), ("AN12", 14.2), ("IL76", 23.7),
        ("E3CF", 26.9), ("E6",   26.9), ("P3",   11.1), ("P8",   11.1),
    ])
});

/// WTC-based fallback.
static EF_WTC: LazyLock<HashMap<&'static str, f64>> = LazyLock::new(|| {
    HashMap::from([("L", 1.5), ("M", 8.0), ("H", 22.0), ("J", 43.5)])
});

/// ADS-B emitter category fallback.
static EF_CAT: LazyLock<HashMap<&'static str, f64>> = LazyLock::new(|| {
    HashMap::from([
        ("A1", 1.2), ("A2", 3.5), ("A3", 9.0), ("A4", 13.9),
        ("A5", 22.0), ("A6", 22.0), ("A7", 0.5),
        ("B1", 0.0), ("B2", 0.1), ("B4", 0.0), ("B6", 0.1),
        ("C1", 0.0), ("C3", 0.0),
    ])
});

/// Default emission factor when nothing is known (kg CO₂/km).
pub const EF_DEFAULT: f64 = 7.0;

/// Source of the emission factor used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EfSource {
    Type,
    Wtc,
    Category,
    Default,
}

impl EfSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Wtc => "wtc",
            Self::Category => "cat",
            Self::Default => "default",
        }
    }
}

/// Look up the emission factor for an aircraft.
///
/// Returns `(factor, source)`. Falls through type → WTC → category → default.
pub fn emission_factor(
    type_code: Option<&str>,
    wtc: Option<&str>,
    category: Option<&str>,
) -> (f64, EfSource) {
    if let Some(tc) = type_code {
        if let Some(&ef) = EF_TYPE.get(tc) {
            return (ef, EfSource::Type);
        }
    }
    if let Some(w) = wtc {
        if let Some(&ef) = EF_WTC.get(w) {
            return (ef, EfSource::Wtc);
        }
    }
    if let Some(c) = category {
        if let Some(&ef) = EF_CAT.get(c) {
            return (ef, EfSource::Category);
        }
    }
    (EF_DEFAULT, EfSource::Default)
}

/// Derive a size-class bucket from emission factor.
pub fn size_class_from_ef(ef: f64) -> &'static str {
    if ef >= 40.0 {
        "super"
    } else if ef >= 12.0 {
        "heavy"
    } else if ef >= 3.0 {
        "medium"
    } else {
        "light"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haversine_known_distance() {
        // London → Paris ≈ 344 km
        let d = haversine_km(51.5074, -0.1278, 48.8566, 2.3522);
        assert!((d - 344.0).abs() < 2.0, "got {d}");
    }

    #[test]
    fn ef_lookup_chain() {
        let (ef, src) = emission_factor(Some("A320"), None, None);
        assert_eq!(ef, 9.5);
        assert_eq!(src, EfSource::Type);

        let (ef, src) = emission_factor(Some("UNKNOWN"), Some("H"), None);
        assert_eq!(ef, 22.0);
        assert_eq!(src, EfSource::Wtc);

        let (ef, src) = emission_factor(None, None, Some("A3"));
        assert_eq!(ef, 9.0);
        assert_eq!(src, EfSource::Category);

        let (ef, src) = emission_factor(None, None, None);
        assert_eq!(ef, EF_DEFAULT);
        assert_eq!(src, EfSource::Default);
    }

    #[test]
    fn size_class_buckets() {
        assert_eq!(size_class_from_ef(43.5), "super");
        assert_eq!(size_class_from_ef(20.0), "heavy");
        assert_eq!(size_class_from_ef(9.5), "medium");
        assert_eq!(size_class_from_ef(1.5), "light");
    }
}
