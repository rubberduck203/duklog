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

    /// Parses a band from its ADIF string representation.
    ///
    /// Accepts the same strings returned by [`adif_str`](Self::adif_str),
    /// case-insensitively.
    pub fn from_adif_str(s: &str) -> Option<Band> {
        match s.to_uppercase().as_str() {
            "160M" => Some(Band::M160),
            "80M" => Some(Band::M80),
            "60M" => Some(Band::M60),
            "40M" => Some(Band::M40),
            "30M" => Some(Band::M30),
            "20M" => Some(Band::M20),
            "17M" => Some(Band::M17),
            "15M" => Some(Band::M15),
            "12M" => Some(Band::M12),
            "10M" => Some(Band::M10),
            "6M" => Some(Band::M6),
            "2M" => Some(Band::M2),
            "70CM" => Some(Band::Cm70),
            _ => None,
        }
    }

    /// Returns the band that contains `freq_khz`, or `None` if the frequency
    /// does not fall within any amateur allocation supported by this enum.
    ///
    /// Ranges are the ADIF v3.1.4 standard band edges.
    pub fn from_frequency_khz(freq_khz: u32) -> Option<Band> {
        match freq_khz {
            1_800..=2_000 => Some(Band::M160),
            3_500..=4_000 => Some(Band::M80),
            5_060..=5_450 => Some(Band::M60),
            7_000..=7_300 => Some(Band::M40),
            10_100..=10_150 => Some(Band::M30),
            14_000..=14_350 => Some(Band::M20),
            18_068..=18_168 => Some(Band::M17),
            21_000..=21_450 => Some(Band::M15),
            24_890..=24_990 => Some(Band::M12),
            28_000..=29_700 => Some(Band::M10),
            50_000..=54_000 => Some(Band::M6),
            144_000..=148_000 => Some(Band::M2),
            420_000..=450_000 => Some(Band::Cm70),
            _ => None,
        }
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
    use quickcheck::quickcheck;

    use super::*;

    quickcheck! {
        fn from_frequency_khz_returns_valid_band_or_none(freq: u32) -> bool {
            match Band::from_frequency_khz(freq) {
                Some(band) => Band::all().contains(&band),
                None => true,
            }
        }
    }

    #[test]
    fn from_frequency_khz_known_frequencies() {
        assert_eq!(Band::from_frequency_khz(1_800), Some(Band::M160));
        assert_eq!(Band::from_frequency_khz(1_900), Some(Band::M160));
        assert_eq!(Band::from_frequency_khz(2_000), Some(Band::M160));
        assert_eq!(Band::from_frequency_khz(3_500), Some(Band::M80));
        assert_eq!(Band::from_frequency_khz(3_750), Some(Band::M80));
        assert_eq!(Band::from_frequency_khz(4_000), Some(Band::M80));
        assert_eq!(Band::from_frequency_khz(5_060), Some(Band::M60));
        assert_eq!(Band::from_frequency_khz(5_450), Some(Band::M60));
        assert_eq!(Band::from_frequency_khz(7_000), Some(Band::M40));
        assert_eq!(Band::from_frequency_khz(7_200), Some(Band::M40));
        assert_eq!(Band::from_frequency_khz(7_300), Some(Band::M40));
        assert_eq!(Band::from_frequency_khz(10_100), Some(Band::M30));
        assert_eq!(Band::from_frequency_khz(10_150), Some(Band::M30));
        assert_eq!(Band::from_frequency_khz(14_000), Some(Band::M20));
        assert_eq!(Band::from_frequency_khz(14_225), Some(Band::M20));
        assert_eq!(Band::from_frequency_khz(14_350), Some(Band::M20));
        assert_eq!(Band::from_frequency_khz(18_068), Some(Band::M17));
        assert_eq!(Band::from_frequency_khz(18_168), Some(Band::M17));
        assert_eq!(Band::from_frequency_khz(21_000), Some(Band::M15));
        assert_eq!(Band::from_frequency_khz(21_450), Some(Band::M15));
        assert_eq!(Band::from_frequency_khz(24_890), Some(Band::M12));
        assert_eq!(Band::from_frequency_khz(24_990), Some(Band::M12));
        assert_eq!(Band::from_frequency_khz(28_000), Some(Band::M10));
        assert_eq!(Band::from_frequency_khz(29_700), Some(Band::M10));
        assert_eq!(Band::from_frequency_khz(50_000), Some(Band::M6));
        assert_eq!(Band::from_frequency_khz(54_000), Some(Band::M6));
        assert_eq!(Band::from_frequency_khz(144_000), Some(Band::M2));
        assert_eq!(Band::from_frequency_khz(148_000), Some(Band::M2));
        assert_eq!(Band::from_frequency_khz(420_000), Some(Band::Cm70));
        assert_eq!(Band::from_frequency_khz(450_000), Some(Band::Cm70));
    }

    #[test]
    fn from_frequency_khz_gaps_return_none() {
        assert_eq!(Band::from_frequency_khz(0), None);
        assert_eq!(Band::from_frequency_khz(1_799), None);
        assert_eq!(Band::from_frequency_khz(2_001), None);
        assert_eq!(Band::from_frequency_khz(3_499), None);
        assert_eq!(Band::from_frequency_khz(4_001), None);
        assert_eq!(Band::from_frequency_khz(5_059), None);
        assert_eq!(Band::from_frequency_khz(5_451), None);
        assert_eq!(Band::from_frequency_khz(6_999), None);
        assert_eq!(Band::from_frequency_khz(7_301), None);
        assert_eq!(Band::from_frequency_khz(10_099), None);
        assert_eq!(Band::from_frequency_khz(10_151), None);
        assert_eq!(Band::from_frequency_khz(13_999), None);
        assert_eq!(Band::from_frequency_khz(14_351), None);
        assert_eq!(Band::from_frequency_khz(29_701), None);
        assert_eq!(Band::from_frequency_khz(54_001), None);
        assert_eq!(Band::from_frequency_khz(419_999), None);
        assert_eq!(Band::from_frequency_khz(450_001), None);
        assert_eq!(Band::from_frequency_khz(u32::MAX), None);
    }

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
