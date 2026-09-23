//! Advanced setting definitions, GTK controls, and Aether argument collection.

use crate::{profile::*, psiphon_setup};
use adw::prelude::*;
use adw::ApplicationWindow;
use gtk::{
    Align, Box as GtkBox, Button, DropDown, Entry, Frame, Grid, Label, Orientation, ProgressBar,
    StringList, Switch,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(super) struct Choice {
    pub(super) label: &'static str,
    pub(super) args: &'static [&'static str],
    pub(super) value: &'static str,
}

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Text(&'static str),
    Secret(&'static str),
    Environment(&'static str),
    EnvironmentChoice(&'static str, &'static [Choice]),
    Path(&'static str),
    Toggle(&'static str, Option<&'static str>),
    Choice(&'static [Choice]),
}

#[derive(Clone, Copy)]
pub(super) enum Condition {
    Always,
    Protocol(&'static [&'static str]),
    Feature(&'static str, &'static [&'static str]),
}

pub(super) struct Spec {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) hint: &'static str,
    pub(super) default_value: &'static str,
    pub(super) kind: Kind,
    pub(super) condition: Condition,
}

#[derive(Clone)]
pub(super) enum Input {
    Entry(Entry),
    Path(Entry, Button),
    Switch(Switch),
    Choice(DropDown, &'static [Choice]),
}

pub(super) struct Binding {
    pub(super) spec: Spec,
    pub(super) row: GtkBox,
    pub(super) input: Input,
}

pub(super) struct Section {
    pub(super) frame: Frame,
    pub(super) bindings: Vec<Binding>,
}

pub(super) struct PsiphonSetupUi {
    pub(super) frame: Frame,
    pub(super) install: Button,
    pub(super) status: Label,
    pub(super) progress: ProgressBar,
}
#[derive(Default)]
pub(super) struct FormState {
    pub(super) values: HashMap<&'static str, String>,
    pub(super) args: Vec<String>,
    pub(super) environment: Vec<(String, String)>,
    pub(super) log_level: String,
    pub(super) profile: ConnectionProfile,
}
pub(super) fn spec(id: &'static str, label: &'static str, hint: &'static str, kind: Kind) -> Spec {
    Spec {
        id,
        label,
        hint,
        default_value: "",
        kind,
        condition: Condition::Always,
    }
}

pub(super) fn conditional(mut spec: Spec, condition: Condition) -> Spec {
    spec.condition = condition;
    spec
}

pub(super) fn text(
    id: &'static str,
    label: &'static str,
    flag: &'static str,
    hint: &'static str,
) -> Spec {
    spec(id, label, hint, Kind::Text(flag))
}

pub(super) fn env_text(
    id: &'static str,
    label: &'static str,
    key: &'static str,
    hint: &'static str,
) -> Spec {
    spec(id, label, hint, Kind::Environment(key))
}

pub(super) fn toggle(
    id: &'static str,
    label: &'static str,
    flag: &'static str,
    off: Option<&'static str>,
) -> Spec {
    spec(id, label, "", Kind::Toggle(flag, off))
}

pub(super) fn dropdown(id: &'static str, label: &'static str, choices: &'static [Choice]) -> Spec {
    spec(id, label, "", Kind::Choice(choices))
}

pub(super) fn env_dropdown(
    id: &'static str,
    label: &'static str,
    key: &'static str,
    choices: &'static [Choice],
) -> Spec {
    spec(id, label, "", Kind::EnvironmentChoice(key, choices))
}
pub(super) fn append_field(grid: &Grid, row_index: i32, spec: Spec) -> Binding {
    let row = GtkBox::new(Orientation::Horizontal, 12);
    row.set_margin_top(4);
    row.set_margin_bottom(4);
    let title = Label::new(Some(spec.label));
    title.set_halign(Align::Start);
    title.set_hexpand(true);
    title.set_wrap(true);
    title.set_max_width_chars(34);
    title.set_tooltip_text((!spec.hint.is_empty()).then_some(spec.hint));
    row.append(&title);
    let input = match spec.kind {
        Kind::Text(_) | Kind::Secret(_) | Kind::Environment(_) => {
            let entry = Entry::new();
            entry.set_width_chars(25);
            if let Kind::Secret(_) = spec.kind {
                entry.set_visibility(false);
            } else if !spec.default_value.is_empty() {
                entry.set_text(spec.default_value);
            }
            if !spec.hint.is_empty() {
                entry.set_placeholder_text(Some(spec.hint));
            }
            row.append(&entry);
            Input::Entry(entry)
        }
        Kind::Path(_) => {
            let entry = Entry::new();
            entry.set_width_chars(25);
            entry.set_hexpand(true);
            entry.set_placeholder_text(Some("Optional - uses the installed client or PATH"));
            if let Some(path) = psiphon_setup::installed_client() {
                entry.set_text(&path.to_string_lossy());
            }
            row.append(&entry);
            let browse = Button::with_label("Choose");
            browse.set_tooltip_text(Some("Choose an existing Psiphon client"));
            browse.add_css_class("flat");
            row.append(&browse);
            Input::Path(entry, browse)
        }
        Kind::Toggle(_, _) => {
            let switch = Switch::new();
            switch.set_valign(Align::Center);
            row.append(&switch);
            Input::Switch(switch)
        }
        Kind::Choice(choices) | Kind::EnvironmentChoice(_, choices) => {
            let labels: Vec<_> = choices.iter().map(|choice| choice.label).collect();
            let list = StringList::new(&labels);
            let dropdown = DropDown::new(Some(list), gtk::Expression::NONE);
            dropdown.set_selected(0);
            dropdown.set_hexpand(false);
            row.append(&dropdown);
            Input::Choice(dropdown, choices)
        }
    };
    grid.attach(&row, 0, row_index, 1, 1);
    Binding { spec, row, input }
}

pub(super) fn append_section(
    parent: &GtkBox,
    title: &str,
    description: &str,
    specs: Vec<Spec>,
) -> Section {
    let frame = Frame::new(None);
    frame.add_css_class("card");
    let content = GtkBox::new(Orientation::Vertical, 6);
    content.set_margin_top(14);
    content.set_margin_bottom(14);
    content.set_margin_start(16);
    content.set_margin_end(16);
    let heading = Label::new(Some(title));
    heading.add_css_class("heading");
    heading.set_halign(Align::Start);
    content.append(&heading);
    if !description.is_empty() {
        let description = Label::new(Some(description));
        description.add_css_class("dim-label");
        description.set_wrap(true);
        description.set_halign(Align::Start);
        content.append(&description);
    }
    let grid = Grid::new();
    grid.set_row_spacing(2);
    grid.set_column_spacing(12);
    let bindings = specs
        .into_iter()
        .enumerate()
        .map(|(index, spec)| append_field(&grid, index as i32, spec))
        .collect();
    content.append(&grid);
    frame.set_child(Some(&content));
    parent.append(&frame);
    Section { frame, bindings }
}
pub(super) fn form_values(sections: &[Section]) -> FormState {
    let mut state = FormState {
        log_level: "info".into(),
        ..Default::default()
    };
    for binding in sections.iter().flat_map(|section| &section.bindings) {
        let value = match &binding.input {
            Input::Entry(entry) => entry.text().trim().to_owned(),
            Input::Path(entry, _) => entry.text().trim().to_owned(),
            Input::Switch(switch) => switch.is_active().to_string(),
            Input::Choice(dropdown, choices) => choices
                .get(dropdown.selected() as usize)
                .map(|choice| choice.value.to_owned())
                .unwrap_or_default(),
        };
        state.values.insert(binding.spec.id, value.clone());
    }
    state.profile = ConnectionProfile {
        mode: ProfileMode::parse(
            state
                .values
                .get("profile_mode")
                .map(String::as_str)
                .unwrap_or("masque"),
        ),
        scan: ScanProfile::parse(
            state
                .values
                .get("scan")
                .map(String::as_str)
                .unwrap_or("balanced"),
        ),
        obfuscation: ObfuscationProfile::parse(
            state
                .values
                .get("noize")
                .map(String::as_str)
                .unwrap_or("default"),
        ),
        direction: ChainDirection::parse(
            state
                .values
                .get("chain_direction")
                .map(String::as_str)
                .unwrap_or("inner"),
        ),
        reuse_last_gateway: state
            .values
            .get("quick_reconnect")
            .is_none_or(|value| value == "yes"),
    };
    state
        .values
        .insert("protocol", state.profile.mode.transport().into());
    state.values.insert(
        "tor_mode",
        match state.profile.mode {
            ProfileMode::TorOnly => "only",
            ProfileMode::TorWarp if state.profile.direction == ChainDirection::TorThroughWarp => {
                "reverse"
            }
            ProfileMode::TorWarp => "chain",
            _ => "off",
        }
        .into(),
    );
    state.values.insert(
        "psiphon_mode",
        match state.profile.mode {
            ProfileMode::PsiphonOnly => "only",
            ProfileMode::PsiphonWarp
                if state.profile.direction == ChainDirection::TorThroughWarp =>
            {
                "reverse"
            }
            ProfileMode::PsiphonWarp => "chain",
            _ => "off",
        }
        .into(),
    );
    state.args.extend(state.profile.args());
    for binding in sections.iter().flat_map(|section| &section.bindings) {
        if !is_visible(binding.spec.condition, &state) {
            continue;
        }
        let value = state
            .values
            .get(binding.spec.id)
            .cloned()
            .unwrap_or_default();
        match binding.spec.kind {
            Kind::Text(flag) | Kind::Secret(flag) if !value.is_empty() => {
                state.args.push(flag.into());
                state.args.push(value.clone());
            }
            Kind::Environment(key) if !value.is_empty() => {
                state.environment.push((key.into(), value.clone()));
            }
            Kind::EnvironmentChoice(key, choices) => {
                if let Some(choice) = choices.iter().find(|choice| choice.value == value) {
                    state.environment.push((key.into(), choice.value.into()));
                }
            }
            Kind::Path(flag) => {
                let explicit = (!value.is_empty()).then(|| PathBuf::from(&value));
                if let Ok(Some(path)) = psiphon_setup::resolve_client_path(
                    explicit.as_deref(),
                    &psiphon_setup::managed_client_path(),
                ) {
                    state.args.push(flag.into());
                    state.args.push(path.to_string_lossy().into_owned());
                }
            }
            Kind::Toggle(on, off) => {
                if value == "true" {
                    state.args.push(on.into());
                } else if let Some(off) = off {
                    state.args.push(off.into());
                }
            }
            Kind::Choice(choices) => {
                if matches!(
                    binding.spec.id,
                    "profile_mode" | "chain_direction" | "scan" | "noize" | "quick_reconnect"
                ) {
                    continue;
                }
                if let Some(choice) = choices.iter().find(|choice| choice.value == value) {
                    state
                        .args
                        .extend(choice.args.iter().map(|arg| (*arg).into()));
                }
            }
            _ => {}
        }
        if binding.spec.id == "log_level" {
            state.log_level = value;
        }
    }
    state
}

pub(super) fn is_visible(condition: Condition, state: &FormState) -> bool {
    // Keep advanced controls in sync with the active protocol and tunnel features.
    match condition {
        Condition::Always => true,
        Condition::Protocol(protocols) => protocols.contains(
            &state
                .values
                .get("protocol")
                .map(String::as_str)
                .unwrap_or("masque"),
        ),
        Condition::Feature(id, values) => {
            values.contains(&state.values.get(id).map(String::as_str).unwrap_or("off"))
        }
    }
}

pub(super) fn refresh_visibility(sections: &[Section], state: &FormState) {
    for section in sections {
        let mut any = false;
        for binding in &section.bindings {
            let visible = is_visible(binding.spec.condition, state);
            binding.row.set_visible(visible);
            any |= visible;
        }
        section.frame.set_visible(any);
    }
}

pub(super) fn find_path_entry(sections: &[Section], id: &str) -> Option<Entry> {
    sections
        .iter()
        .flat_map(|section| &section.bindings)
        .find(|binding| binding.spec.id == id)
        .and_then(|binding| match &binding.input {
            Input::Path(entry, _) => Some(entry.clone()),
            _ => None,
        })
}

pub(super) fn find_choice_dropdown(sections: &[Section], id: &str) -> Option<DropDown> {
    sections
        .iter()
        .flat_map(|section| &section.bindings)
        .find(|binding| binding.spec.id == id)
        .and_then(|binding| match &binding.input {
            Input::Choice(dropdown, _) => Some(dropdown.clone()),
            _ => None,
        })
}
pub(super) fn choose_psiphon_client(parent: &ApplicationWindow, entry: &Entry, status: &Label) {
    let dialog = gtk::FileChooserNative::new(
        Some("Choose Psiphon client"),
        Some(parent),
        gtk::FileChooserAction::Open,
        Some("Select"),
        Some("Cancel"),
    );
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Psiphon client"));
    filter.add_pattern("psiphon-tunnel-core*");
    filter.add_pattern("psiphon*");
    dialog.add_filter(&filter);
    let entry = entry.clone();
    let status = status.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            match dialog.file().and_then(|file| file.path()) {
                Some(path) if psiphon_setup::is_executable_client(&path) => {
                    entry.set_text(&path.to_string_lossy());
                    status.set_text(&format!("Using {}", path.display()));
                }
                Some(path) => status.set_text(&format!(
                    "Selected file is not an executable Psiphon client: {}",
                    path.display()
                )),
                None => status.set_text("The selected file path could not be read."),
            }
        }
        dialog.hide();
    });
    dialog.show();
}

pub(super) fn entry_spec(id: &'static str, label: &'static str, flag: &'static str) -> Spec {
    text(id, label, flag, "")
}

pub(super) fn entry_spec_default(
    id: &'static str,
    label: &'static str,
    flag: &'static str,
    default_value: &'static str,
) -> Spec {
    let mut spec = entry_spec(id, label, flag);
    spec.default_value = default_value;
    spec
}

pub(super) fn path_spec(id: &'static str, label: &'static str, flag: &'static str) -> Spec {
    spec(id, label, "", Kind::Path(flag))
}

pub(super) fn append_psiphon_setup(parent: &GtkBox) -> PsiphonSetupUi {
    let frame = Frame::new(None);
    frame.add_css_class("card");
    let content = GtkBox::new(Orientation::Vertical, 6);
    content.set_margin_top(10);
    content.set_margin_bottom(10);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let row = GtkBox::new(Orientation::Horizontal, 8);
    let status = Label::new(Some("Psiphon client is missing."));
    status.set_halign(Align::Start);
    status.set_hexpand(true);
    status.set_wrap(true);
    let installed = psiphon_setup::installed_client();
    if installed.is_some() {
        status.set_text("Psiphon client is ready.");
    }
    let install = Button::with_label("Install Psiphon");
    install.add_css_class("suggested-action");
    install.set_sensitive(installed.is_none());
    install.set_tooltip_text(Some(if installed.is_some() {
        "Psiphon client is installed"
    } else {
        "Build and install the Psiphon client"
    }));
    let progress = ProgressBar::new();
    progress.set_show_text(true);
    progress.set_text(Some("Building Psiphon client"));
    progress.set_visible(false);
    row.append(&status);
    row.append(&install);
    content.append(&row);
    content.append(&progress);
    frame.set_child(Some(&content));
    parent.append(&frame);
    PsiphonSetupUi {
        frame,
        install,
        status,
        progress,
    }
}

pub(super) fn psiphon_client_available(binary_entry: Option<&Entry>) -> bool {
    let explicit = binary_entry
        .map(Entry::text)
        .filter(|path| !path.is_empty())
        .map(|path| PathBuf::from(path.as_str()));
    psiphon_setup::resolve_client_path(explicit.as_deref(), &psiphon_setup::managed_client_path())
        .is_ok_and(|client| client.is_some())
}

pub(super) fn should_show_psiphon_setup(mode: ProfileMode, binary_entry: Option<&Entry>) -> bool {
    matches!(mode, ProfileMode::PsiphonOnly | ProfileMode::PsiphonWarp)
        && !psiphon_client_available(binary_entry)
}

pub(super) fn maybe_prompt_psiphon_install(
    window: &ApplicationWindow,
    mode: ProfileMode,
    binary_entry: Option<&Entry>,
    install: &Button,
) {
    if !matches!(mode, ProfileMode::PsiphonOnly | ProfileMode::PsiphonWarp)
        || psiphon_client_available(binary_entry)
    {
        return;
    }

    let dialog = gtk::MessageDialog::new(
        Some(window),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Question,
        gtk::ButtonsType::None,
        "Psiphon client required",
    );
    dialog.format_secondary_text(
        Some("This profile needs the Psiphon client. Install it now? Requires Go, Git, Bash, and internet access."),
    );
    dialog
        .add_button("Not now", gtk::ResponseType::Cancel)
        .set_tooltip_text(Some("Close this prompt"));
    dialog
        .add_button("Install Psiphon", gtk::ResponseType::Accept)
        .set_tooltip_text(Some("Build and install the Psiphon client"));
    dialog.set_default_response(gtk::ResponseType::Accept);
    let install = install.clone();
    dialog.connect_response(move |dialog, response| {
        dialog.hide();
        if response == gtk::ResponseType::Accept {
            install.emit_clicked();
        }
    });
    dialog.present();
}

#[allow(clippy::vec_init_then_push)]
pub(super) fn build_settings(parent: &GtkBox) -> Vec<Section> {
    let mut sections = Vec::new();
    sections.push(append_section(
        parent,
        "Connection",
        "",
        vec![
            dropdown("profile_mode", "Connection profile", MODES),
            conditional(
                dropdown("scan", "Gateway scan", SCANS),
                Condition::Feature(
                    "profile_mode",
                    &[
                        "masque",
                        "wg",
                        "gool",
                        "mim",
                        "tor_warp",
                        "psiphon_warp",
                        "zero_trust",
                    ],
                ),
            ),
            conditional(
                dropdown("chain_direction", "Chain direction", CHAIN_DIRECTIONS),
                Condition::Feature("profile_mode", &["tor_warp", "psiphon_warp"]),
            ),
            dropdown("ip", "IP family", IP_FAMILIES),
            conditional(
                dropdown("noize", "Obfuscation profile", NOIZE),
                Condition::Feature(
                    "profile_mode",
                    &[
                        "masque",
                        "wg",
                        "gool",
                        "mim",
                        "tor_warp",
                        "psiphon_warp",
                        "zero_trust",
                    ],
                ),
            ),
            entry_spec_default("bind", "SOCKS5 listen address", "--bind", "127.0.0.1:1819"),
            entry_spec("http_proxy", "Optional HTTP proxy", "--http-proxy"),
            entry_spec("upstream", "Upstream proxy URL", "--upstream"),
            conditional(
                dropdown("quick_reconnect", "Reconnect behavior", QUICK_RECONNECT),
                Condition::Feature(
                    "profile_mode",
                    &[
                        "masque",
                        "wg",
                        "gool",
                        "mim",
                        "tor_warp",
                        "psiphon_warp",
                        "zero_trust",
                    ],
                ),
            ),
            toggle(
                "stats",
                "Show traffic and uptime statistics",
                "--stats",
                None,
            ),
            entry_spec_default(
                "stats_secs",
                "Statistics interval (seconds)",
                "--stats-secs",
                "60",
            ),
            entry_spec("exit_loc", "Allowed / blocked exit countries", "--exit-loc"),
            entry_spec_default(
                "exit_loc_secs",
                "Exit country recheck interval",
                "--exit-loc-secs",
                "60",
            ),
            entry_spec("mark", "Linux firewall mark", "--mark"),
        ],
    ));
    sections.push(append_section(
        parent,
        "Known endpoints",
        "Leave blank to let Aether discover endpoints.",
        vec![
            entry_spec("peer", "MASQUE / WireGuard peer", "--peer"),
            entry_spec("wg_peer", "WireGuard peer", "--wg-peer"),
            conditional(
                entry_spec("wiw_outer", "gool outer endpoint", "--wiw-outer"),
                Condition::Protocol(&["gool"]),
            ),
            conditional(
                entry_spec("wiw_inner", "gool inner endpoint", "--wiw-inner"),
                Condition::Protocol(&["gool"]),
            ),
            conditional(
                entry_spec("wiw_peers", "gool endpoint pair", "--wiw-peers"),
                Condition::Protocol(&["gool"]),
            ),
            conditional(
                entry_spec("mim_outer", "mim outer endpoint", "--mim-outer"),
                Condition::Protocol(&["mim"]),
            ),
            conditional(
                entry_spec("mim_inner", "mim inner endpoint", "--mim-inner"),
                Condition::Protocol(&["mim"]),
            ),
            conditional(
                entry_spec("mim_peers", "mim endpoint pair", "--mim-peers"),
                Condition::Protocol(&["mim"]),
            ),
        ],
    ));
    sections.push(append_section(
        parent,
        "MASQUE transport",
        "HTTP/2 uses TCP; fragmentation applies only to HTTP/2.",
        vec![
            conditional(
                dropdown("carrier", "Carrier", CARRIER),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                toggle("no_quic_v2", "Disable QUIC v2 opener", "--no-quic-v2", None),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                entry_spec("h2_peer", "HTTP/2 peer override", "--h2-peer"),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                entry_spec("ech", "Encrypted Client Hello", "--ech"),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                toggle("fragment", "Fragment TLS ClientHello", "--fragment", None),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                entry_spec_default("fragment_size", "Fragment size", "--fragment-size", "16-32"),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                entry_spec_default(
                    "fragment_delay",
                    "Fragment delay",
                    "--fragment-delay",
                    "2-10",
                ),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                toggle(
                    "no_data_check",
                    "Skip data-plane validation",
                    "--no-data-check",
                    None,
                ),
                Condition::Always,
            ),
            conditional(
                entry_spec_default(
                    "validate_secs",
                    "Validation timeout",
                    "--validate-secs",
                    "10",
                ),
                Condition::Always,
            ),
            conditional(
                entry_spec_default("startup_secs", "Startup deadline", "--startup-secs", "30"),
                Condition::Protocol(&["masque", "mim"]),
            ),
            conditional(
                entry_spec_default("reconnect_secs", "Reconnect delay", "--reconnect-secs", "2"),
                Condition::Always,
            ),
            conditional(
                entry_spec_default("dns", "Tunnel DNS resolvers", "--dns", "1.1.1.1,1.0.0.1"),
                Condition::Protocol(&["masque", "mim"]),
            ),
        ],
    ));
    sections.push(append_section(
        parent,
        "WireGuard",
        "",
        vec![
            conditional(
                entry_spec_default(
                    "keepalive",
                    "Persistent keepalive interval",
                    "--keepalive",
                    "5",
                ),
                Condition::Protocol(&["wg", "gool"]),
            ),
            conditional(
                toggle(
                    "no_profile_retry",
                    "Disable profile retries",
                    "--no-profile-retry",
                    None,
                ),
                Condition::Protocol(&["wg", "gool"]),
            ),
        ],
    ));
    sections.push(append_section(
        parent,
        "Tor",
        "",
        vec![
            conditional(
                dropdown("tor_bridges", "Bridge discovery", BRIDGES),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                text(
                    "tor_bridge",
                    "Custom bridge line",
                    "--tor-bridge",
                    "One bridge line",
                ),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec("tor_bridge_file", "Bridge file", "--tor-bridge-file"),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec_default("tor_relays", "Relay source / count", "--tor-relays", "auto"),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                dropdown("tor_relay_ports", "Relay ports", RELAY_PORTS),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec_default(
                    "tor_bind",
                    "Tor SOCKS address",
                    "--tor-bind",
                    "127.0.0.1:1820",
                ),
                Condition::Feature("tor_mode", &["chain", "reverse"]),
            ),
            conditional(
                entry_spec("tor_http", "Tor HTTP proxy address", "--tor-http"),
                Condition::Feature("tor_mode", &["chain", "reverse"]),
            ),
            conditional(
                entry_spec("tor_dir", "Tor data directory", "--tor-dir"),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec("tor_pt", "Pluggable transport binary", "--tor-pt"),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec(
                    "tor_pt_dir",
                    "Pluggable transport directory",
                    "--tor-pt-dir",
                ),
                Condition::Feature("tor_mode", &["chain", "reverse", "only"]),
            ),
        ],
    ));
    sections.push(append_section(
        parent,
        "Psiphon",
        "",
        vec![
            conditional(
                entry_spec("psiphon_config", "Psiphon JSON config", "--psiphon-config"),
                Condition::Feature("psiphon_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                dropdown("psiphon_shape", "Psiphon route", PSIPHON_SHAPE),
                Condition::Feature("psiphon_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec("psiphon_cdn_ips", "CDN edge addresses", "--psiphon-cdn-ips"),
                Condition::Feature("psiphon_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec("psiphon_cdn_sni", "CDN server names", "--psiphon-cdn-sni"),
                Condition::Feature("psiphon_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec_default(
                    "psiphon_bind",
                    "Psiphon SOCKS address",
                    "--psiphon-bind",
                    "127.0.0.1:1821",
                ),
                Condition::Feature("psiphon_mode", &["chain", "reverse"]),
            ),
            conditional(
                entry_spec(
                    "psiphon_http",
                    "Psiphon HTTP proxy address",
                    "--psiphon-http",
                ),
                Condition::Feature("psiphon_mode", &["chain", "reverse"]),
            ),
            conditional(
                entry_spec("psiphon_region", "Psiphon exit region", "--psiphon-region"),
                Condition::Feature("psiphon_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                entry_spec("psiphon_dir", "Psiphon data directory", "--psiphon-dir"),
                Condition::Feature("psiphon_mode", &["chain", "reverse", "only"]),
            ),
            conditional(
                path_spec("psiphon_bin", "Psiphon binary path", "--psiphon-bin"),
                Condition::Feature("psiphon_mode", &["chain", "reverse", "only"]),
            ),
        ],
    ));
    sections.push(append_section(
        parent,
        "Zero Trust",
        "Credentials are not saved.",
        vec![
            conditional(
                entry_spec("team", "Team name", "--team"),
                Condition::Feature("profile_mode", &["zero_trust"]),
            ),
            conditional(
                entry_spec("access_id", "Service token client ID", "--access-id"),
                Condition::Feature("profile_mode", &["zero_trust"]),
            ),
            conditional(
                spec(
                    "access_secret",
                    "Service token secret",
                    "",
                    Kind::Secret("--access-secret"),
                ),
                Condition::Feature("profile_mode", &["zero_trust"]),
            ),
            conditional(
                spec(
                    "access_email",
                    "One-time-code email",
                    "",
                    Kind::Text("--access-email"),
                ),
                Condition::Feature("profile_mode", &["zero_trust"]),
            ),
            conditional(
                spec(
                    "access_token",
                    "Enrollment token",
                    "",
                    Kind::Secret("--access-token"),
                ),
                Condition::Feature("profile_mode", &["zero_trust"]),
            ),
            conditional(
                toggle(
                    "gateway",
                    "Send web traffic through organization gateway",
                    "--gateway",
                    None,
                ),
                Condition::Feature("profile_mode", &["zero_trust"]),
            ),
        ],
    ));
    sections.push(append_section(
        parent,
        "Traffic routing",
        "Block rules take priority.",
        vec![
            entry_spec("route_block", "Block these destinations", "--route-block"),
            entry_spec(
                "route_direct",
                "Send these destinations directly",
                "--route-direct",
            ),
            entry_spec("routes", "Routing rules file", "--routes"),
        ],
    ));
    sections.push(append_section(
        parent,
        "Identity and configuration",
        "Aether stores identity data in config files.",
        vec![
            entry_spec("config", "Base config file", "--config"),
            entry_spec("wg_config", "WireGuard config file", "--wg-config"),
            entry_spec("masque_config", "MASQUE config file", "--masque-config"),
        ],
    ));
    sections.push(append_section(
        parent,
        "Diagnostics and performance",
        "",
        vec![
            entry_spec("tls_groups", "TLS key-share groups", "--tls-groups"),
            dropdown("perf", "Resource profile", PERF),
            dropdown("log_level", "Aether log level", LOG_LEVEL),
        ],
    ));
    sections.push(append_section(
        parent,
        "Expert environment",
        "",
        vec![
            env_text(
                "tor_direct_secs",
                "Tor direct-connect window",
                "AETHER_TOR_DIRECT_SECS",
                "",
            ),
            env_text(
                "tor_stall_secs",
                "Tor bridge stall timeout",
                "AETHER_TOR_STALL_SECS",
                "",
            ),
            env_text(
                "tor_bridge_secs",
                "Tor bridge timeout",
                "AETHER_TOR_BRIDGE_SECS",
                "",
            ),
            env_text(
                "tor_country",
                "Tor bridge country",
                "AETHER_TOR_COUNTRY",
                "",
            ),
            env_text(
                "tor_check",
                "Tor reachability check",
                "AETHER_TOR_CHECK",
                "",
            ),
            env_dropdown("tor_log", "Tor log level", "AETHER_TOR_LOG", TOR_LOG_LEVEL),
            env_text(
                "psiphon_ready_secs",
                "Psiphon readiness timeout",
                "AETHER_PSIPHON_READY_SECS",
                "",
            ),
            env_text(
                "route_sniff",
                "Route SNI sniffing (0 disables)",
                "AETHER_ROUTE_SNIFF",
                "",
            ),
            env_text(
                "route_sniff_ms",
                "Route SNI sniff timeout",
                "AETHER_ROUTE_SNIFF_MS",
                "",
            ),
            env_text(
                "wg_endpoint_cooldown",
                "WireGuard endpoint cooldown",
                "AETHER_WG_ENDPOINT_COOLDOWN_SECS",
                "",
            ),
            env_text(
                "wg_stale_secs",
                "WireGuard stale timeout",
                "AETHER_WG_STALE_SECS",
                "",
            ),
            env_text(
                "h2_keepalive_secs",
                "HTTP/2 keepalive interval",
                "AETHER_MASQUE_H2_KEEPALIVE_SECS",
                "",
            ),
            env_text(
                "h2_keepalive_timeout",
                "HTTP/2 keepalive timeout",
                "AETHER_MASQUE_H2_KEEPALIVE_TIMEOUT_SECS",
                "",
            ),
            env_text(
                "ironclad_port",
                "Ironclad HTTP check port",
                "AETHER_IRONCLAD_PORT",
                "",
            ),
            env_text(
                "max_clients",
                "Maximum simultaneous proxy clients",
                "AETHER_MAX_CLIENTS",
                "",
            ),
            env_text(
                "half_close_secs",
                "TCP half-close timeout",
                "AETHER_HALF_CLOSE_SECS",
                "",
            ),
            env_text(
                "tcp_keepalive_secs",
                "TCP keepalive timeout",
                "AETHER_TCP_KEEPALIVE_SECS",
                "",
            ),
            env_text(
                "tcp_connect_secs",
                "TCP connection timeout",
                "AETHER_TCP_CONNECT_SECS",
                "",
            ),
            env_text(
                "reprovision",
                "Disable automatic reprovisioning (0)",
                "AETHER_REPROVISION",
                "",
            ),
        ],
    ));
    sections
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_visibility_tracks_protocol_and_feature_modes() {
        let mut state = FormState::default();
        let wireguard = Condition::Protocol(&["wg"]);
        let tor_settings = Condition::Feature("tor_mode", &["chain", "reverse", "only"]);

        assert!(!is_visible(wireguard, &state));
        assert!(!is_visible(tor_settings, &state));
        state.values.insert("protocol", "wg".into());
        state.values.insert("tor_mode", "chain".into());
        assert!(is_visible(wireguard, &state));
        assert!(is_visible(tor_settings, &state));
    }
}
