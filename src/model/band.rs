use std::fmt;

use serde::{Deserialize, Serialize};

/// Amateur radio frequency band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Band {
    M160,
    M80,
    M60,
    M40,
    M30,
    #[default]
    M20,
    M17,
    M15,
    M12,
    M10,
    M6,
    M2,
    Cm70,
}

static ALL_BANDS: &[Band] = &[
    Band::M160,
    Band::M80,
    Band::M60,
    Band::M40,
    Band::M30,
    Band::M20,
    Band::M17,
    Band::M15,
    Band::M12,
    Band::M10,
    Band::M6,
    Band::M2,
    Band::Cm70,
];

impl Band {
    /// Returns the ADIF string representation of this band.
    pub fn adif_str(&self) -> &'static str {
        match self {
            Band::M160 => "160M",
            Band::M80 => "80M",
            Band::M60 => "60M",
            Band::M40 => "40M",
            Band::M30 => "30M",
            Band::M20 => "20M",
            Band::M17 => "17M",
            Band::M15 => "15M",
            Band::M12 => "12M",
            Band::M10 => "10M",
            Band::M6 => "6M",
            Band::M2 => "2M",
            Band::Cm70 => "70CM",
        }
    }

    /// Returns all bands in wavelength order (longest to shortest).
    pub fn all() -> &'static [Band] {
        ALL_BANDS
    }
}

#[mutants::skip]
impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.adif_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adif_str_all_bands() {
        assert_eq!(Band::M160.adif_str(), "160M");
        assert_eq!(Band::M80.adif_str(), "80M");
        assert_eq!(Band::M60.adif_str(), "60M");
        assert_eq!(Band::M40.adif_str(), "40M");
        assert_eq!(Band::M30.adif_str(), "30M");
        assert_eq!(Band::M20.adif_str(), "20M");
        assert_eq!(Band::M17.adif_str(), "17M");
        assert_eq!(Band::M15.adif_str(), "15M");
        assert_eq!(Band::M12.adif_str(), "12M");
        assert_eq!(Band::M10.adif_str(), "10M");
        assert_eq!(Band::M6.adif_str(), "6M");
        assert_eq!(Band::M2.adif_str(), "2M");
        assert_eq!(Band::Cm70.adif_str(), "70CM");
    }

    #[test]
    fn all_returns_13_bands() {
        assert_eq!(Band::all().len(), 13);
    }

    #[test]
    fn all_starts_with_160m_ends_with_70cm() {
        assert_eq!(Band::all().first(), Some(&Band::M160));
        assert_eq!(Band::all().last(), Some(&Band::Cm70));
    }

    #[test]
    fn default_is_20m() {
        assert_eq!(Band::default(), Band::M20);
    }

    #[test]
    fn serde_round_trip() {
        for band in Band::all() {
            let json = serde_json::to_string(band).unwrap();
            let deserialized: Band = serde_json::from_str(&json).unwrap();
            assert_eq!(*band, deserialized);
        }
    }
}
