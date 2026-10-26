//! Connection profiles and their non-secret saved preferences.

use crate::settings::{form_values, refresh_visibility, Choice, FormState, Input, Section};
use gtk::DropDown;
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProfileMode {
    Masque,
    WireGuard,
    Gool,
    MasqueInMasque,
    TorOnly,
    TorWarp,
    PsiphonOnly,
    PsiphonWarp,
    ZeroTrust,
}

impl ProfileMode {
    pub(super) fn parse(value: &str) -> Self {
        match value {
            "wg" => Self::WireGuard,
            "gool" => Self::Gool,
            "mim" => Self::MasqueInMasque,
            "tor_only" => Self::TorOnly,
            "tor_warp" => Self::TorWarp,
            "psiphon_only" => Self::PsiphonOnly,
            "psiphon_warp" => Self::PsiphonWarp,
            "zero_trust" => Self::ZeroTrust,
            _ => Self::Masque,
        }
    }

    pub(super) fn value(self) -> &'static str {
        match self {
            Self::Masque => "masque",
            Self::WireGuard => "wg",
            Self::Gool => "gool",
            Self::MasqueInMasque => "mim",
            Self::TorOnly => "tor_only",
            Self::TorWarp => "tor_warp",
            Self::PsiphonOnly => "psiphon_only",
            Self::PsiphonWarp => "psiphon_warp",
            Self::ZeroTrust => "zero_trust",
        }
    }

    pub(super) fn transport(self) -> &'static str {
        match self {
            Self::WireGuard => "wg",
            Self::Gool => "gool",
            Self::MasqueInMasque => "mim",
            _ => "masque",
        }
    }

    pub(super) fn args(self) -> &'static [&'static str] {
        match self {
            Self::Masque | Self::TorWarp | Self::PsiphonWarp | Self::ZeroTrust => &[],
            Self::WireGuard => &["--wg"],
            Self::Gool => &["--gool-classic"],
            Self::MasqueInMasque => &["--mim"],
            Self::TorOnly => &["--tor-only"],
            Self::PsiphonOnly => &["--psiphon-only"],
        }
    }

    pub(super) fn uses_warp(self) -> bool {
        !matches!(self, Self::TorOnly | Self::PsiphonOnly)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ChainDirection {
    WarpThroughTor,
    TorThroughWarp,
}

impl ChainDirection {
    pub(super) fn parse(value: &str) -> Self {
        if value == "outer" {
            Self::TorThroughWarp
        } else {
            Self::WarpThroughTor
        }
    }

    pub(super) fn value(self) -> &'static str {
        match self {
            Self::WarpThroughTor => "inner",
            Self::TorThroughWarp => "outer",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScanProfile {
    Balanced,
    Turbo,
    Thorough,
    Verified,
    Ironclad,
}

impl ScanProfile {
    pub(super) fn parse(value: &str) -> Self {
        match value {
            "turbo" => Self::Turbo,
            "thorough" => Self::Thorough,
            "verified" => Self::Verified,
            "ironclad" => Self::Ironclad,
            _ => Self::Balanced,
        }
    }

    pub(super) fn value(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Turbo => "turbo",
            Self::Thorough => "thorough",
            Self::Verified => "verified",
            Self::Ironclad => "ironclad",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObfuscationProfile {
    Default,
    Firewall,
    Balanced,
    Aggressive,
    Gfw,
    Light,
    Off,
}

impl ObfuscationProfile {
    pub(super) fn parse(value: &str) -> Self {
        match value {
            "firewall" => Self::Firewall,
            "balanced" => Self::Balanced,
            "aggressive" => Self::Aggressive,
            "gfw" => Self::Gfw,
            "light" => Self::Light,
            "off" => Self::Off,
            _ => Self::Default,
        }
    }

    pub(super) fn value(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Firewall => "firewall",
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
            Self::Gfw => "gfw",
            Self::Light => "light",
            Self::Off => "off",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ConnectionProfile {
    pub(super) mode: ProfileMode,
    pub(super) scan: ScanProfile,
    pub(super) obfuscation: ObfuscationProfile,
    pub(super) direction: ChainDirection,
    pub(super) reuse_last_gateway: bool,
}

impl ConnectionProfile {
    // Standalone profiles omit WARP-only scan and reconnect options.
    pub(super) fn args(self) -> Vec<String> {
        let mut args = vec!["--protocol".into(), self.mode.transport().into()];
        args.extend(self.mode.args().iter().map(|arg| (*arg).into()));
        match self.mode {
            ProfileMode::TorWarp => args.push(
                if self.direction == ChainDirection::TorThroughWarp {
                    "--tor-reverse"
                } else {
                    "--tor"
                }
                .into(),
            ),
            ProfileMode::PsiphonWarp => args.push(
                if self.direction == ChainDirection::TorThroughWarp {
                    "--psiphon-reverse"
                } else {
                    "--psiphon"
                }
                .into(),
            ),
            _ => {}
        }
        if self.mode.uses_warp() {
            args.push(
                if self.reuse_last_gateway {
                    "--quick-reconnect"
                } else {
                    "--no-quick-reconnect"
                }
                .into(),
            );
            args.extend(["--scan".into(), self.scan.value().into()]);
            if self.obfuscation != ObfuscationProfile::Default {
                args.extend(["--noize".into(), self.obfuscation.value().into()]);
            }
        }
        args
    }
}

impl Default for ConnectionProfile {
    fn default() -> Self {
        Self {
            mode: ProfileMode::Masque,
            scan: ScanProfile::Balanced,
            obfuscation: ObfuscationProfile::Default,
            direction: ChainDirection::WarpThroughTor,
            reuse_last_gateway: true,
        }
    }
}

pub(super) const MODES: &[Choice] = &[
    Choice {
        label: "MASQUE - modern",
        args: &["--masque"],
        value: "masque",
    },
    Choice {
        label: "WireGuard - classic",
        args: &["--wg"],
        value: "wg",
    },
    Choice {
        label: "WARP-in-WARP - gool",
        args: &["--gool-classic"],
        value: "gool",
    },
    Choice {
        label: "MASQUE-in-MASQUE",
        args: &["--mim"],
        value: "mim",
    },
    Choice {
        label: "Tor only",
        args: &["--tor-only"],
        value: "tor_only",
    },
    Choice {
        label: "Tor - WARP",
        args: &[],
        value: "tor_warp",
    },
    Choice {
        label: "Psiphon only",
        args: &["--psiphon-only"],
        value: "psiphon_only",
    },
    Choice {
        label: "Psiphon - WARP",
        args: &[],
        value: "psiphon_warp",
    },
    Choice {
        label: "Zero Trust - organization",
        args: &[],
        value: "zero_trust",
    },
];
pub(super) const CHAIN_DIRECTIONS: &[Choice] = &[
    Choice {
        label: "WARP first - Tor or Psiphon",
        args: &[],
        value: "inner",
    },
    Choice {
        label: "Tor or Psiphon first - WARP",
        args: &[],
        value: "outer",
    },
];

pub(super) const SCANS: &[Choice] = &[
    Choice {
        label: "Turbo",
        args: &["--scan", "turbo"],
        value: "turbo",
    },
    Choice {
        label: "Balanced",
        args: &["--scan", "balanced"],
        value: "balanced",
    },
    Choice {
        label: "Thorough",
        args: &["--scan", "thorough"],
        value: "thorough",
    },
    Choice {
        label: "Verified",
        args: &["--scan", "verified"],
        value: "verified",
    },
    Choice {
        label: "Ironclad",
        args: &["--scan", "ironclad"],
        value: "ironclad",
    },
];
pub(super) const IP_FAMILIES: &[Choice] = &[
    Choice {
        label: "IPv4",
        args: &["--ip", "v4"],
        value: "v4",
    },
    Choice {
        label: "IPv6",
        args: &["--ip", "v6"],
        value: "v6",
    },
    Choice {
        label: "IPv4 and IPv6",
        args: &["--ip", "both"],
        value: "both",
    },
];
pub(super) const NOIZE: &[Choice] = &[
    Choice {
        label: "Protocol default",
        args: &[],
        value: "default",
    },
    Choice {
        label: "Firewall",
        args: &["--noize", "firewall"],
        value: "firewall",
    },
    Choice {
        label: "Balanced",
        args: &["--noize", "balanced"],
        value: "balanced",
    },
    Choice {
        label: "Aggressive",
        args: &["--noize", "aggressive"],
        value: "aggressive",
    },
    Choice {
        label: "GFW",
        args: &["--noize", "gfw"],
        value: "gfw",
    },
    Choice {
        label: "Light",
        args: &["--noize", "light"],
        value: "light",
    },
    Choice {
        label: "Off",
        args: &["--noize", "off"],
        value: "off",
    },
];
pub(super) const QUICK_RECONNECT: &[Choice] = &[
    Choice {
        label: "Reuse last successful gateway",
        args: &["--quick-reconnect"],
        value: "yes",
    },
    Choice {
        label: "Always scan for a fresh gateway",
        args: &["--no-quick-reconnect"],
        value: "no",
    },
];
pub(super) const CARRIER: &[Choice] = &[
    Choice {
        label: "HTTP/3 (QUIC)",
        args: &["--h3"],
        value: "h3",
    },
    Choice {
        label: "HTTP/2 (TCP)",
        args: &["--h2"],
        value: "h2",
    },
];
pub(super) const BRIDGES: &[Choice] = &[
    Choice {
        label: "Automatic",
        args: &["--tor-bridges"],
        value: "auto",
    },
    Choice {
        label: "No bridges",
        args: &["--no-tor-bridges"],
        value: "off",
    },
];
pub(super) const PSIPHON_SHAPE: &[Choice] = &[
    Choice {
        label: "Automatic",
        args: &["--psiphon-mode", "auto"],
        value: "auto",
    },
    Choice {
        label: "CDN",
        args: &["--psiphon-mode", "cdn"],
        value: "cdn",
    },
    Choice {
        label: "Direct",
        args: &["--psiphon-mode", "direct"],
        value: "direct",
    },
];
pub(super) const RELAY_PORTS: &[Choice] = &[
    Choice {
        label: "Web ports only",
        args: &["--tor-relay-ports", "web"],
        value: "web",
    },
    Choice {
        label: "Any port",
        args: &["--tor-relay-ports", "any"],
        value: "any",
    },
];
pub(super) const PERF: &[Choice] = &[
    Choice {
        label: "Automatic",
        args: &[],
        value: "auto",
    },
    Choice {
        label: "Low",
        args: &["--perf", "low"],
        value: "low",
    },
    Choice {
        label: "Medium",
        args: &["--perf", "medium"],
        value: "medium",
    },
    Choice {
        label: "High",
        args: &["--perf", "high"],
        value: "high",
    },
];
pub(super) const LOG_LEVEL: &[Choice] = &[
    Choice {
        label: "Info",
        args: &["--log-level", "info"],
        value: "info",
    },
    Choice {
        label: "Debug",
        args: &["--log-level", "debug"],
        value: "debug",
    },
    Choice {
        label: "Trace",
        args: &["--log-level", "trace"],
        value: "trace",
    },
    Choice {
        label: "Warn",
        args: &["--log-level", "warn"],
        value: "warn",
    },
    Choice {
        label: "Error",
        args: &["--log-level", "error"],
        value: "error",
    },
];
pub(super) const TOR_LOG_LEVEL: &[Choice] = &[
    Choice {
        label: "Info",
        args: &[],
        value: "info",
    },
    Choice {
        label: "Debug",
        args: &[],
        value: "debug",
    },
    Choice {
        label: "Trace",
        args: &[],
        value: "trace",
    },
];
pub(super) const REPROVISION: &[Choice] = &[
    Choice {
        label: "Yes - replace refused identities automatically",
        args: &[],
        value: "1",
    },
    Choice {
        label: "No - keep refused identities",
        args: &[],
        value: "0",
    },
];

pub(super) fn profile_path() -> PathBuf {
    glib::user_config_dir().join("aether-gtk/profile")
}
pub(super) fn load_profile() -> ConnectionProfile {
    read_profile(&profile_path())
}

pub(super) fn read_profile(path: &Path) -> ConnectionProfile {
    let values = fs::read_to_string(path).unwrap_or_default();
    let mut lines = values.lines();
    ConnectionProfile {
        mode: ProfileMode::parse(lines.next().unwrap_or("masque")),
        scan: ScanProfile::parse(lines.next().unwrap_or("balanced")),
        obfuscation: ObfuscationProfile::parse(lines.next().unwrap_or("default")),
        direction: ChainDirection::parse(lines.next().unwrap_or("inner")),
        reuse_last_gateway: lines.next().unwrap_or("yes") != "no",
    }
}

// Only the quick-profile choices are stored here; credentials remain in memory.
pub(super) fn save_profile(path: &Path, profile: ConnectionProfile) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        path,
        format!(
            "{}\n{}\n{}\n{}\n{}\n",
            profile.mode.value(),
            profile.scan.value(),
            profile.obfuscation.value(),
            profile.direction.value(),
            if profile.reuse_last_gateway {
                "yes"
            } else {
                "no"
            }
        ),
    );
}
pub(super) fn set_choice(sections: &[Section], id: &str, value: &str) {
    for binding in sections.iter().flat_map(|section| &section.bindings) {
        if binding.spec.id != id {
            continue;
        }
        if let Input::Choice(dropdown, choices) = &binding.input {
            if let Some(index) = choices.iter().position(|choice| choice.value == value) {
                dropdown.set_selected(index as u32);
            }
        }
        break;
    }
}

pub(super) fn set_profile_widgets(profile: ConnectionProfile, widgets: &[(&str, DropDown)]) {
    let values = [
        ("profile_mode", profile.mode.value()),
        ("scan", profile.scan.value()),
        ("noize", profile.obfuscation.value()),
        ("chain_direction", profile.direction.value()),
        (
            "quick_reconnect",
            if profile.reuse_last_gateway {
                "yes"
            } else {
                "no"
            },
        ),
    ];
    for (id, value) in values {
        let choices = match id {
            "profile_mode" => MODES,
            "scan" => SCANS,
            "noize" => NOIZE,
            "quick_reconnect" => QUICK_RECONNECT,
            _ => CHAIN_DIRECTIONS,
        };
        if let Some(index) = choices.iter().position(|choice| choice.value == value) {
            for (_, dropdown) in widgets.iter().filter(|(widget_id, _)| *widget_id == id) {
                dropdown.set_selected(index as u32);
            }
        }
    }
}

pub(super) fn profile_choices(id: &str) -> &'static [Choice] {
    match id {
        "profile_mode" => MODES,
        "scan" => SCANS,
        "noize" => NOIZE,
        "quick_reconnect" => QUICK_RECONNECT,
        _ => CHAIN_DIRECTIONS,
    }
}

pub(super) fn sync_profile_change(
    id: &'static str,
    source: &DropDown,
    sections: &[Section],
    widgets: &[(&str, DropDown)],
    state: &RefCell<FormState>,
    syncing: &Cell<bool>,
) {
    // The guard prevents the home controls and advanced settings from echoing each other.
    if syncing.get() {
        return;
    }
    let Some(choice) = profile_choices(id).get(source.selected() as usize) else {
        return;
    };
    syncing.set(true);
    set_choice(sections, id, choice.value);
    for (widget_id, dropdown) in widgets.iter().filter(|(widget_id, _)| *widget_id == id) {
        let _ = widget_id;
        if let Some(index) = profile_choices(id)
            .iter()
            .position(|candidate| candidate.value == choice.value)
        {
            dropdown.set_selected(index as u32);
        }
    }
    let updated = form_values(sections);
    save_profile(&profile_path(), updated.profile);
    refresh_visibility(sections, &updated);
    *state.borrow_mut() = updated;
    syncing.set(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_nine_profiles_map_to_aether_arguments_and_chain_direction() {
        let modes = [
            "masque",
            "wg",
            "gool",
            "mim",
            "tor_only",
            "tor_warp",
            "psiphon_only",
            "psiphon_warp",
            "zero_trust",
        ];
        assert_eq!(MODES.len(), 9);
        for (choice, value) in MODES.iter().zip(modes) {
            assert_eq!(choice.value, value);
            assert_eq!(ProfileMode::parse(choice.value).value(), value);
        }

        let profile_args = |mode, direction| {
            ConnectionProfile {
                mode,
                direction,
                ..ConnectionProfile::default()
            }
            .args()
        };
        let assert_protocol = |mode, protocol| {
            let args = profile_args(mode, ChainDirection::WarpThroughTor);
            assert_eq!(args.first().map(String::as_str), Some("--protocol"));
            assert_eq!(args.get(1).map(String::as_str), Some(protocol));
        };
        for (mode, protocol) in [
            (ProfileMode::Masque, "masque"),
            (ProfileMode::WireGuard, "wg"),
            (ProfileMode::Gool, "gool"),
            (ProfileMode::MasqueInMasque, "mim"),
            (ProfileMode::TorOnly, "masque"),
            (ProfileMode::TorWarp, "masque"),
            (ProfileMode::PsiphonOnly, "masque"),
            (ProfileMode::PsiphonWarp, "masque"),
            (ProfileMode::ZeroTrust, "masque"),
        ] {
            assert_protocol(mode, protocol);
        }
        assert!(
            profile_args(ProfileMode::Masque, ChainDirection::WarpThroughTor)
                .contains(&"--scan".into())
        );
        assert!(
            profile_args(ProfileMode::Masque, ChainDirection::WarpThroughTor)
                .contains(&"--quick-reconnect".into())
        );
        let scan_fresh = ConnectionProfile {
            reuse_last_gateway: false,
            ..ConnectionProfile::default()
        }
        .args();
        assert!(scan_fresh.contains(&"--no-quick-reconnect".into()));
        assert!(
            profile_args(ProfileMode::WireGuard, ChainDirection::WarpThroughTor)
                .contains(&"--wg".into())
        );
        assert!(
            profile_args(ProfileMode::Gool, ChainDirection::WarpThroughTor)
                .contains(&"--gool-classic".into())
        );
        assert!(
            profile_args(ProfileMode::MasqueInMasque, ChainDirection::WarpThroughTor)
                .contains(&"--mim".into())
        );
        assert_eq!(
            profile_args(ProfileMode::TorOnly, ChainDirection::WarpThroughTor),
            ["--protocol", "masque", "--tor-only"]
        );
        assert!(
            profile_args(ProfileMode::TorWarp, ChainDirection::WarpThroughTor)
                .contains(&"--tor".into())
        );
        assert!(
            profile_args(ProfileMode::TorWarp, ChainDirection::TorThroughWarp)
                .contains(&"--tor-reverse".into())
        );
        assert_eq!(
            profile_args(ProfileMode::PsiphonOnly, ChainDirection::WarpThroughTor),
            ["--protocol", "masque", "--psiphon-only"]
        );
        assert!(
            profile_args(ProfileMode::PsiphonWarp, ChainDirection::WarpThroughTor)
                .contains(&"--psiphon".into())
        );
        assert!(
            profile_args(ProfileMode::PsiphonWarp, ChainDirection::TorThroughWarp)
                .contains(&"--psiphon-reverse".into())
        );
        assert!(
            profile_args(ProfileMode::ZeroTrust, ChainDirection::WarpThroughTor)
                .contains(&"--scan".into())
        );
    }

    #[test]
    fn profile_preferences_persist_only_non_secret_choices() {
        let path = std::env::temp_dir().join(format!(
            "aether-gtk-profile-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let profile = ConnectionProfile {
            mode: ProfileMode::PsiphonWarp,
            scan: ScanProfile::Verified,
            obfuscation: ObfuscationProfile::Gfw,
            direction: ChainDirection::TorThroughWarp,
            reuse_last_gateway: false,
        };
        save_profile(&path, profile);
        let lines = fs::read_to_string(&path).unwrap();
        assert_eq!(lines.lines().count(), 5);
        assert!(!lines.contains("secret") && !lines.contains("token") && !lines.contains("team"));
        assert_eq!(read_profile(&path), profile);
        let _ = fs::remove_file(path);
    }
}
