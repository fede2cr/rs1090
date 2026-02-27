//! icao_country — map ICAO 24-bit hex addresses to registration country
//!
//! Based on ICAO Annex 10, Vol III, Chapter 9 allocation table.
//! Same ranges as tar1090's flags.js.

/// Country info resolved from an ICAO hex address.
#[derive(Debug, Clone)]
pub struct IcaoCountry {
    pub country: &'static str,
    pub code: &'static str, // ISO 3166-1 alpha-2
}

struct Range {
    start: u32,
    end: u32,
    country: &'static str,
    code: &'static str,
}

/// Sorted by start address. Overlapping sub-ranges (e.g. UK territories
/// carved from the UK block) are listed first so the linear scan finds
/// them before the parent block.
static RANGES: &[Range] = &[
    Range { start: 0x004000, end: 0x0047FF, country: "Zimbabwe", code: "zw" },
    Range { start: 0x006000, end: 0x006FFF, country: "Mozambique", code: "mz" },
    Range { start: 0x008000, end: 0x00FFFF, country: "South Africa", code: "za" },
    Range { start: 0x010000, end: 0x017FFF, country: "Egypt", code: "eg" },
    Range { start: 0x018000, end: 0x01FFFF, country: "Libya", code: "ly" },
    Range { start: 0x020000, end: 0x027FFF, country: "Morocco", code: "ma" },
    Range { start: 0x028000, end: 0x02FFFF, country: "Tunisia", code: "tn" },
    Range { start: 0x030000, end: 0x0307FF, country: "Botswana", code: "bw" },
    Range { start: 0x032000, end: 0x032FFF, country: "Burundi", code: "bi" },
    Range { start: 0x034000, end: 0x034FFF, country: "Cameroon", code: "cm" },
    Range { start: 0x035000, end: 0x0357FF, country: "Comoros", code: "km" },
    Range { start: 0x036000, end: 0x036FFF, country: "Republic of the Congo", code: "cg" },
    Range { start: 0x038000, end: 0x038FFF, country: "Côte d'Ivoire", code: "ci" },
    Range { start: 0x03E000, end: 0x03EFFF, country: "Gabon", code: "ga" },
    Range { start: 0x040000, end: 0x040FFF, country: "Ethiopia", code: "et" },
    Range { start: 0x042000, end: 0x042FFF, country: "Equatorial Guinea", code: "gq" },
    Range { start: 0x044000, end: 0x044FFF, country: "Ghana", code: "gh" },
    Range { start: 0x046000, end: 0x046FFF, country: "Guinea", code: "gn" },
    Range { start: 0x048000, end: 0x0487FF, country: "Guinea-Bissau", code: "gw" },
    Range { start: 0x04A000, end: 0x04A7FF, country: "Lesotho", code: "ls" },
    Range { start: 0x04C000, end: 0x04CFFF, country: "Kenya", code: "ke" },
    Range { start: 0x050000, end: 0x050FFF, country: "Liberia", code: "lr" },
    Range { start: 0x054000, end: 0x054FFF, country: "Madagascar", code: "mg" },
    Range { start: 0x058000, end: 0x058FFF, country: "Malawi", code: "mw" },
    Range { start: 0x05A000, end: 0x05A7FF, country: "Maldives", code: "mv" },
    Range { start: 0x05C000, end: 0x05CFFF, country: "Mali", code: "ml" },
    Range { start: 0x05E000, end: 0x05E7FF, country: "Mauritania", code: "mr" },
    Range { start: 0x060000, end: 0x0607FF, country: "Mauritius", code: "mu" },
    Range { start: 0x062000, end: 0x062FFF, country: "Niger", code: "ne" },
    Range { start: 0x064000, end: 0x064FFF, country: "Nigeria", code: "ng" },
    Range { start: 0x068000, end: 0x068FFF, country: "Uganda", code: "ug" },
    Range { start: 0x06A000, end: 0x06AFFF, country: "Qatar", code: "qa" },
    Range { start: 0x06C000, end: 0x06CFFF, country: "Central African Republic", code: "cf" },
    Range { start: 0x06E000, end: 0x06EFFF, country: "Rwanda", code: "rw" },
    Range { start: 0x070000, end: 0x070FFF, country: "Senegal", code: "sn" },
    Range { start: 0x074000, end: 0x0747FF, country: "Seychelles", code: "sc" },
    Range { start: 0x076000, end: 0x0767FF, country: "Sierra Leone", code: "sl" },
    Range { start: 0x078000, end: 0x078FFF, country: "Somalia", code: "so" },
    Range { start: 0x07A000, end: 0x07A7FF, country: "Eswatini", code: "sz" },
    Range { start: 0x07C000, end: 0x07CFFF, country: "Sudan", code: "sd" },
    Range { start: 0x080000, end: 0x080FFF, country: "Tanzania", code: "tz" },
    Range { start: 0x084000, end: 0x084FFF, country: "Chad", code: "td" },
    Range { start: 0x088000, end: 0x088FFF, country: "Togo", code: "tg" },
    Range { start: 0x08A000, end: 0x08AFFF, country: "Zambia", code: "zm" },
    Range { start: 0x08C000, end: 0x08CFFF, country: "DR Congo", code: "cd" },
    Range { start: 0x090000, end: 0x090FFF, country: "Angola", code: "ao" },
    Range { start: 0x094000, end: 0x0947FF, country: "Benin", code: "bj" },
    Range { start: 0x096000, end: 0x0967FF, country: "Cabo Verde", code: "cv" },
    Range { start: 0x098000, end: 0x0987FF, country: "Djibouti", code: "dj" },
    Range { start: 0x09A000, end: 0x09AFFF, country: "Gambia", code: "gm" },
    Range { start: 0x09C000, end: 0x09CFFF, country: "Burkina Faso", code: "bf" },
    Range { start: 0x09E000, end: 0x09E7FF, country: "São Tomé and Príncipe", code: "st" },
    Range { start: 0x0A0000, end: 0x0A7FFF, country: "Algeria", code: "dz" },
    Range { start: 0x0A8000, end: 0x0A8FFF, country: "Bahamas", code: "bs" },
    Range { start: 0x0AA000, end: 0x0AA7FF, country: "Barbados", code: "bb" },
    Range { start: 0x0AB000, end: 0x0AB7FF, country: "Belize", code: "bz" },
    Range { start: 0x0AC000, end: 0x0ADFFF, country: "Colombia", code: "co" },
    Range { start: 0x0AE000, end: 0x0AEFFF, country: "Costa Rica", code: "cr" },
    Range { start: 0x0B0000, end: 0x0B0FFF, country: "Cuba", code: "cu" },
    Range { start: 0x0B2000, end: 0x0B2FFF, country: "El Salvador", code: "sv" },
    Range { start: 0x0B4000, end: 0x0B4FFF, country: "Guatemala", code: "gt" },
    Range { start: 0x0B6000, end: 0x0B6FFF, country: "Guyana", code: "gy" },
    Range { start: 0x0B8000, end: 0x0B8FFF, country: "Haiti", code: "ht" },
    Range { start: 0x0BA000, end: 0x0BAFFF, country: "Honduras", code: "hn" },
    Range { start: 0x0BC000, end: 0x0BC7FF, country: "Saint Vincent and the Grenadines", code: "vc" },
    Range { start: 0x0BE000, end: 0x0BEFFF, country: "Jamaica", code: "jm" },
    Range { start: 0x0C0000, end: 0x0C0FFF, country: "Nicaragua", code: "ni" },
    Range { start: 0x0C2000, end: 0x0C2FFF, country: "Panama", code: "pa" },
    Range { start: 0x0C4000, end: 0x0C4FFF, country: "Dominican Republic", code: "do" },
    Range { start: 0x0C6000, end: 0x0C6FFF, country: "Trinidad and Tobago", code: "tt" },
    Range { start: 0x0C8000, end: 0x0C8FFF, country: "Suriname", code: "sr" },
    Range { start: 0x0CA000, end: 0x0CA7FF, country: "Antigua and Barbuda", code: "ag" },
    Range { start: 0x0CC000, end: 0x0CC7FF, country: "Grenada", code: "gd" },
    Range { start: 0x0D0000, end: 0x0D7FFF, country: "Mexico", code: "mx" },
    Range { start: 0x0D8000, end: 0x0DFFFF, country: "Venezuela", code: "ve" },
    Range { start: 0x100000, end: 0x1FFFFF, country: "Russia", code: "ru" },
    Range { start: 0x201000, end: 0x2017FF, country: "Namibia", code: "na" },
    Range { start: 0x202000, end: 0x2027FF, country: "Eritrea", code: "er" },
    Range { start: 0x300000, end: 0x33FFFF, country: "Italy", code: "it" },
    Range { start: 0x340000, end: 0x37FFFF, country: "Spain", code: "es" },
    Range { start: 0x380000, end: 0x3BFFFF, country: "France", code: "fr" },
    Range { start: 0x3C0000, end: 0x3FFFFF, country: "Germany", code: "de" },
    // UK territories — carved out before the UK catch-all
    Range { start: 0x400000, end: 0x4001BF, country: "Bermuda", code: "bm" },
    Range { start: 0x4001C0, end: 0x4001FF, country: "Cayman Islands", code: "ky" },
    Range { start: 0x400300, end: 0x4003FF, country: "Turks and Caicos Islands", code: "tc" },
    Range { start: 0x424135, end: 0x4241F2, country: "Cayman Islands", code: "ky" },
    Range { start: 0x424200, end: 0x4246FF, country: "Bermuda", code: "bm" },
    Range { start: 0x424700, end: 0x424899, country: "Cayman Islands", code: "ky" },
    Range { start: 0x424B00, end: 0x424BFF, country: "Isle of Man", code: "im" },
    Range { start: 0x43BE00, end: 0x43BEFF, country: "Bermuda", code: "bm" },
    Range { start: 0x43E700, end: 0x43EAFD, country: "Isle of Man", code: "im" },
    Range { start: 0x43EAFE, end: 0x43EEFF, country: "Guernsey", code: "gg" },
    Range { start: 0x400000, end: 0x43FFFF, country: "United Kingdom", code: "gb" },
    Range { start: 0x440000, end: 0x447FFF, country: "Austria", code: "at" },
    Range { start: 0x448000, end: 0x44FFFF, country: "Belgium", code: "be" },
    Range { start: 0x450000, end: 0x457FFF, country: "Bulgaria", code: "bg" },
    Range { start: 0x458000, end: 0x45FFFF, country: "Denmark", code: "dk" },
    Range { start: 0x460000, end: 0x467FFF, country: "Finland", code: "fi" },
    Range { start: 0x468000, end: 0x46FFFF, country: "Greece", code: "gr" },
    Range { start: 0x470000, end: 0x477FFF, country: "Hungary", code: "hu" },
    Range { start: 0x478000, end: 0x47FFFF, country: "Norway", code: "no" },
    Range { start: 0x480000, end: 0x487FFF, country: "Netherlands", code: "nl" },
    Range { start: 0x488000, end: 0x48FFFF, country: "Poland", code: "pl" },
    Range { start: 0x490000, end: 0x497FFF, country: "Portugal", code: "pt" },
    Range { start: 0x498000, end: 0x49FFFF, country: "Czechia", code: "cz" },
    Range { start: 0x4A0000, end: 0x4A7FFF, country: "Romania", code: "ro" },
    Range { start: 0x4A8000, end: 0x4AFFFF, country: "Sweden", code: "se" },
    Range { start: 0x4B0000, end: 0x4B7FFF, country: "Switzerland", code: "ch" },
    Range { start: 0x4B8000, end: 0x4BFFFF, country: "Turkey", code: "tr" },
    Range { start: 0x4C0000, end: 0x4C7FFF, country: "Serbia", code: "rs" },
    Range { start: 0x4C8000, end: 0x4C87FF, country: "Cyprus", code: "cy" },
    Range { start: 0x4CA000, end: 0x4CAFFF, country: "Ireland", code: "ie" },
    Range { start: 0x4CC000, end: 0x4CCFFF, country: "Iceland", code: "is" },
    Range { start: 0x4D0000, end: 0x4D07FF, country: "Luxembourg", code: "lu" },
    Range { start: 0x4D2000, end: 0x4D27FF, country: "Malta", code: "mt" },
    Range { start: 0x4D4000, end: 0x4D47FF, country: "Monaco", code: "mc" },
    Range { start: 0x500000, end: 0x5007FF, country: "San Marino", code: "sm" },
    Range { start: 0x501000, end: 0x5017FF, country: "Albania", code: "al" },
    Range { start: 0x501800, end: 0x501FFF, country: "Croatia", code: "hr" },
    Range { start: 0x502800, end: 0x502FFF, country: "Latvia", code: "lv" },
    Range { start: 0x503800, end: 0x503FFF, country: "Lithuania", code: "lt" },
    Range { start: 0x504800, end: 0x504FFF, country: "Moldova", code: "md" },
    Range { start: 0x505800, end: 0x505FFF, country: "Slovakia", code: "sk" },
    Range { start: 0x506800, end: 0x506FFF, country: "Slovenia", code: "si" },
    Range { start: 0x507800, end: 0x507FFF, country: "Uzbekistan", code: "uz" },
    Range { start: 0x508000, end: 0x50FFFF, country: "Ukraine", code: "ua" },
    Range { start: 0x510000, end: 0x5107FF, country: "Belarus", code: "by" },
    Range { start: 0x511000, end: 0x5117FF, country: "Estonia", code: "ee" },
    Range { start: 0x512000, end: 0x5127FF, country: "North Macedonia", code: "mk" },
    Range { start: 0x513000, end: 0x5137FF, country: "Bosnia and Herzegovina", code: "ba" },
    Range { start: 0x514000, end: 0x5147FF, country: "Georgia", code: "ge" },
    Range { start: 0x515000, end: 0x5157FF, country: "Tajikistan", code: "tj" },
    Range { start: 0x516000, end: 0x5167FF, country: "Montenegro", code: "me" },
    Range { start: 0x600000, end: 0x6007FF, country: "Armenia", code: "am" },
    Range { start: 0x600800, end: 0x600FFF, country: "Azerbaijan", code: "az" },
    Range { start: 0x601000, end: 0x6017FF, country: "Kyrgyzstan", code: "kg" },
    Range { start: 0x601800, end: 0x601FFF, country: "Turkmenistan", code: "tm" },
    Range { start: 0x680000, end: 0x6807FF, country: "Bhutan", code: "bt" },
    Range { start: 0x681000, end: 0x6817FF, country: "Micronesia", code: "fm" },
    Range { start: 0x682000, end: 0x6827FF, country: "Mongolia", code: "mn" },
    Range { start: 0x683000, end: 0x6837FF, country: "Kazakhstan", code: "kz" },
    Range { start: 0x684000, end: 0x6847FF, country: "Palau", code: "pw" },
    Range { start: 0x700000, end: 0x700FFF, country: "Afghanistan", code: "af" },
    Range { start: 0x702000, end: 0x702FFF, country: "Bangladesh", code: "bd" },
    Range { start: 0x704000, end: 0x704FFF, country: "Myanmar", code: "mm" },
    Range { start: 0x706000, end: 0x706FFF, country: "Kuwait", code: "kw" },
    Range { start: 0x708000, end: 0x708FFF, country: "Laos", code: "la" },
    Range { start: 0x70A000, end: 0x70AFFF, country: "Nepal", code: "np" },
    Range { start: 0x70C000, end: 0x70C7FF, country: "Oman", code: "om" },
    Range { start: 0x70E000, end: 0x70EFFF, country: "Cambodia", code: "kh" },
    Range { start: 0x710000, end: 0x717FFF, country: "Saudi Arabia", code: "sa" },
    Range { start: 0x718000, end: 0x71FFFF, country: "South Korea", code: "kr" },
    Range { start: 0x720000, end: 0x727FFF, country: "North Korea", code: "kp" },
    Range { start: 0x728000, end: 0x72FFFF, country: "Iraq", code: "iq" },
    Range { start: 0x730000, end: 0x737FFF, country: "Iran", code: "ir" },
    Range { start: 0x738000, end: 0x73FFFF, country: "Israel", code: "il" },
    Range { start: 0x740000, end: 0x747FFF, country: "Jordan", code: "jo" },
    Range { start: 0x748000, end: 0x74FFFF, country: "Lebanon", code: "lb" },
    Range { start: 0x750000, end: 0x757FFF, country: "Malaysia", code: "my" },
    Range { start: 0x758000, end: 0x75FFFF, country: "Philippines", code: "ph" },
    Range { start: 0x760000, end: 0x767FFF, country: "Pakistan", code: "pk" },
    Range { start: 0x768000, end: 0x76FFFF, country: "Singapore", code: "sg" },
    Range { start: 0x770000, end: 0x777FFF, country: "Sri Lanka", code: "lk" },
    Range { start: 0x778000, end: 0x77FFFF, country: "Syria", code: "sy" },
    Range { start: 0x789000, end: 0x789FFF, country: "Hong Kong", code: "hk" },
    Range { start: 0x780000, end: 0x7BFFFF, country: "China", code: "cn" },
    Range { start: 0x7C0000, end: 0x7FFFFF, country: "Australia", code: "au" },
    Range { start: 0x800000, end: 0x83FFFF, country: "India", code: "in" },
    Range { start: 0x840000, end: 0x87FFFF, country: "Japan", code: "jp" },
    Range { start: 0x880000, end: 0x887FFF, country: "Thailand", code: "th" },
    Range { start: 0x888000, end: 0x88FFFF, country: "Viet Nam", code: "vn" },
    Range { start: 0x890000, end: 0x890FFF, country: "Yemen", code: "ye" },
    Range { start: 0x894000, end: 0x894FFF, country: "Bahrain", code: "bh" },
    Range { start: 0x895000, end: 0x8957FF, country: "Brunei", code: "bn" },
    Range { start: 0x896000, end: 0x896FFF, country: "United Arab Emirates", code: "ae" },
    Range { start: 0x897000, end: 0x8977FF, country: "Solomon Islands", code: "sb" },
    Range { start: 0x898000, end: 0x898FFF, country: "Papua New Guinea", code: "pg" },
    Range { start: 0x899000, end: 0x8997FF, country: "Taiwan", code: "tw" },
    Range { start: 0x8A0000, end: 0x8A7FFF, country: "Indonesia", code: "id" },
    Range { start: 0x900000, end: 0x9007FF, country: "Marshall Islands", code: "mh" },
    Range { start: 0x901000, end: 0x9017FF, country: "Cook Islands", code: "ck" },
    Range { start: 0x902000, end: 0x9027FF, country: "Samoa", code: "ws" },
    Range { start: 0xA00000, end: 0xAFFFFF, country: "United States", code: "us" },
    Range { start: 0xC00000, end: 0xC3FFFF, country: "Canada", code: "ca" },
    Range { start: 0xC80000, end: 0xC87FFF, country: "New Zealand", code: "nz" },
    Range { start: 0xC88000, end: 0xC88FFF, country: "Fiji", code: "fj" },
    Range { start: 0xC8A000, end: 0xC8A7FF, country: "Nauru", code: "nr" },
    Range { start: 0xC8C000, end: 0xC8C7FF, country: "Saint Lucia", code: "lc" },
    Range { start: 0xC8D000, end: 0xC8D7FF, country: "Tonga", code: "to" },
    Range { start: 0xC8E000, end: 0xC8E7FF, country: "Kiribati", code: "ki" },
    Range { start: 0xC90000, end: 0xC907FF, country: "Vanuatu", code: "vu" },
    Range { start: 0xC91000, end: 0xC917FF, country: "Andorra", code: "ad" },
    Range { start: 0xC92000, end: 0xC927FF, country: "Dominica", code: "dm" },
    Range { start: 0xC93000, end: 0xC937FF, country: "Saint Kitts and Nevis", code: "kn" },
    Range { start: 0xC94000, end: 0xC947FF, country: "South Sudan", code: "ss" },
    Range { start: 0xC95000, end: 0xC957FF, country: "Timor-Leste", code: "tl" },
    Range { start: 0xC97000, end: 0xC977FF, country: "Tuvalu", code: "tv" },
    Range { start: 0xE00000, end: 0xE3FFFF, country: "Argentina", code: "ar" },
    Range { start: 0xE40000, end: 0xE7FFFF, country: "Brazil", code: "br" },
    Range { start: 0xE80000, end: 0xE80FFF, country: "Chile", code: "cl" },
    Range { start: 0xE84000, end: 0xE84FFF, country: "Ecuador", code: "ec" },
    Range { start: 0xE88000, end: 0xE88FFF, country: "Paraguay", code: "py" },
    Range { start: 0xE8C000, end: 0xE8CFFF, country: "Peru", code: "pe" },
    Range { start: 0xE90000, end: 0xE90FFF, country: "Uruguay", code: "uy" },
    Range { start: 0xE94000, end: 0xE94FFF, country: "Bolivia", code: "bo" },
];

/// Look up the registration country for an ICAO hex string.
///
/// Returns `None` for unassigned or unrecognised addresses.
pub fn lookup(hex: &str) -> Option<IcaoCountry> {
    let addr = u32::from_str_radix(hex, 16).ok()?;
    // Linear scan — the table is ~160 entries, fast enough.
    // Sub-ranges (UK territories, Hong Kong etc.) appear before their
    // parent block, so the first match wins.
    for r in RANGES {
        if addr >= r.start && addr <= r.end {
            return Some(IcaoCountry {
                country: r.country,
                code: r.code,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn germany() {
        let c = lookup("3c6752").unwrap();
        assert_eq!(c.code, "de");
        assert_eq!(c.country, "Germany");
    }

    #[test]
    fn usa() {
        let c = lookup("a12345").unwrap();
        assert_eq!(c.code, "us");
    }

    #[test]
    fn uk() {
        let c = lookup("406a12").unwrap();
        assert_eq!(c.code, "gb");
    }

    #[test]
    fn bermuda_subrange() {
        // Should match Bermuda (0x400000–0x4001BF) before UK
        let c = lookup("400100").unwrap();
        assert_eq!(c.code, "bm");
    }

    #[test]
    fn hong_kong_subrange() {
        let c = lookup("789abc").unwrap();
        assert_eq!(c.code, "hk");
    }

    #[test]
    fn unassigned() {
        assert!(lookup("ffffff").is_none());
    }

    #[test]
    fn brazil() {
        let c = lookup("e45000").unwrap();
        assert_eq!(c.code, "br");
    }
}
