use std::fmt;

use serde::{Deserialize, Serialize};

/// Amateur radio operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Mode {
    #[default]
    Ssb,
    Cw,
    Ft8,
    Ft4,
    Js8,
    Psk31,
    Rtty,
    Fm,
    Am,
    Digi,
}

static ALL_MODES: &[Mode] = &[
    Mode::Ssb,
    Mode::Cw,
    Mode::Ft8,
    Mode::Ft4,
    Mode::Js8,
    Mode::Psk31,
    Mode::Rtty,
    Mode::Fm,
    Mode::Am,
    Mode::Digi,
];

impl Mode {
    /// Returns the ADIF string representation of this mode.
    pub fn adif_str(&self) -> &'static str {
        match self {
            Mode::Ssb => "SSB",
            Mode::Cw => "CW",
            Mode::Ft8 => "FT8",
            Mode::Ft4 => "FT4",
            Mode::Js8 => "JS8",
            Mode::Psk31 => "PSK31",
            Mode::Rtty => "RTTY",
            Mode::Fm => "FM",
            Mode::Am => "AM",
            Mode::Digi => "DIGI",
        }
    }

    /// Returns all modes.
    pub fn all() -> &'static [Mode] {
        ALL_MODES
    }

    /// Returns the default RST (signal report) for this mode.
    ///
    /// `Digi` uses dB reports (`-10`) like FT8/FT4/JS8, as the generic digital
    /// mode in duklog is intended for weak-signal digital modes.
    pub fn default_rst(&self) -> &'static str {
        match self {
            Mode::Ssb | Mode::Fm | Mode::Am => "59",
            Mode::Cw | Mode::Psk31 | Mode::Rtty => "599",
            Mode::Ft8 | Mode::Ft4 | Mode::Js8 | Mode::Digi => "-10",
        }
    }
}

#[mutants::skip]
impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.adif_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adif_str_all_modes() {
        assert_eq!(Mode::Ssb.adif_str(), "SSB");
        assert_eq!(Mode::Cw.adif_str(), "CW");
        assert_eq!(Mode::Ft8.adif_str(), "FT8");
        assert_eq!(Mode::Ft4.adif_str(), "FT4");
        assert_eq!(Mode::Js8.adif_str(), "JS8");
        assert_eq!(Mode::Psk31.adif_str(), "PSK31");
        assert_eq!(Mode::Rtty.adif_str(), "RTTY");
        assert_eq!(Mode::Fm.adif_str(), "FM");
        assert_eq!(Mode::Am.adif_str(), "AM");
        assert_eq!(Mode::Digi.adif_str(), "DIGI");
    }

    #[test]
    fn default_rst_all_modes() {
        assert_eq!(Mode::Ssb.default_rst(), "59");
        assert_eq!(Mode::Cw.default_rst(), "599");
        assert_eq!(Mode::Ft8.default_rst(), "-10");
        assert_eq!(Mode::Ft4.default_rst(), "-10");
        assert_eq!(Mode::Js8.default_rst(), "-10");
        assert_eq!(Mode::Psk31.default_rst(), "599");
        assert_eq!(Mode::Rtty.default_rst(), "599");
        assert_eq!(Mode::Fm.default_rst(), "59");
        assert_eq!(Mode::Am.default_rst(), "59");
        assert_eq!(Mode::Digi.default_rst(), "-10");
    }

    #[test]
    fn all_returns_10_modes() {
        assert_eq!(Mode::all().len(), 10);
    }

    #[test]
    fn default_is_ssb() {
        assert_eq!(Mode::default(), Mode::Ssb);
    }

    #[test]
    fn serde_round_trip() {
        for mode in Mode::all() {
            let json = serde_json::to_string(mode).unwrap();
            let deserialized: Mode = serde_json::from_str(&json).unwrap();
            assert_eq!(*mode, deserialized);
        }
    }
}
