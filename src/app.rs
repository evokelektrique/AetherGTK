//! Application window assembly, appearance, startup, and connection event wiring.

use crate::{connection::*, core, profile::*, psiphon_setup, settings::*};
use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, StyleManager};
use gtk::{
    Align, Box as GtkBox, Button, Frame, Image, Label, Orientation, ScrolledWindow, Spinner, Stack,
    StackSwitcher, Switch, TextView, ToggleButton,
};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

fn configure_theme(manager: &StyleManager, choice: &str) {
    manager.set_color_scheme(theme_scheme(choice));
}

fn theme_scheme(choice: &str) -> adw::ColorScheme {
    match choice {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    }
}

fn theme_path() -> PathBuf {
    glib::user_config_dir().join("aether-gtk/theme")
}

fn load_theme() -> String {
    read_theme(&theme_path())
}

fn read_theme(path: &Path) -> String {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| ["system", "light", "dark"].contains(&value.as_str()))
        .unwrap_or_else(|| "system".into())
}

fn save_theme(choice: &str) {
    write_theme(&theme_path(), choice);
}

fn write_theme(path: &Path, choice: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, choice);
}
fn autostart_path() -> PathBuf {
    glib::user_config_dir().join("autostart/io.github.evokelektrique.AetherGtk.desktop")
}

fn auto_connect_path() -> PathBuf {
    glib::user_config_dir().join("aether-gtk/auto-connect")
}

fn auto_connect_enabled() -> bool {
    fs::read_to_string(auto_connect_path()).is_ok_and(|value| value.trim() == "true")
}

fn set_auto_connect_enabled(enabled: bool) -> std::io::Result<()> {
    let path = auto_connect_path();
    let parent = path
        .parent()
        .expect("auto-connect path should have a parent");
    fs::create_dir_all(parent)?;
    fs::write(path, if enabled { "true" } else { "false" })
}

fn set_autostart_enabled(enabled: bool) -> std::io::Result<()> {
    let path = autostart_path();
    if !enabled {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        };
    }

    let executable = std::env::current_exe()?;
    let executable = executable.to_string_lossy();
    let executable = executable
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    let contents = format!(
        "[Desktop Entry]\nType=Application\nName=Aether\nComment=Start Aether in the background at login\nExec=/usr/bin/env AETHER_GTK_BACKGROUND=1 \"{executable}\"\nIcon=io.github.evokelektrique.AetherGtk\nTerminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
    );
    let parent = path.parent().expect("autostart path should have a parent");
    fs::create_dir_all(parent)?;
    fs::write(path, contents)
}
fn append_autostart_setting(parent: &GtkBox) -> (Switch, Switch, Label) {
    let frame = Frame::new(None);
    frame.add_css_class("card");
    let content = GtkBox::new(Orientation::Vertical, 8);
    content.set_margin_top(14);
    content.set_margin_bottom(14);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let heading = Label::new(Some("Startup"));
    heading.add_css_class("heading");
    heading.set_halign(Align::Start);
    let setting_row = |title: &str, active| {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        let label = Label::new(Some(title));
        label.set_halign(Align::Start);
        let switch = Switch::new();
        switch.set_valign(Align::Center);
        switch.set_active(active);
        label.set_hexpand(true);
        row.append(&label);
        row.append(&switch);
        (row, switch)
    };
    let (startup_row, startup) = setting_row("Start at login", autostart_path().is_file());
    let (auto_connect_row, auto_connect) = setting_row("Connect at login", auto_connect_enabled());
    auto_connect.set_sensitive(startup.is_active());
    content.append(&heading);
    content.append(&startup_row);
    content.append(&auto_connect_row);
    let status = Label::new(None);
    status.set_halign(Align::Start);
    status.add_css_class("caption");
    status.add_css_class("dim-label");
    status.set_visible(false);
    content.append(&status);
    frame.set_child(Some(&content));
    parent.append(&frame);
    (startup, auto_connect, status)
}
fn theme_icon(choice: &str) -> &'static str {
    match choice {
        "light" => "weather-clear-symbolic",
        "dark" => "weather-clear-night-symbolic",
        _ => "preferences-desktop-display-symbolic",
    }
}

fn theme_label(choice: &str) -> &'static str {
    match choice {
        "light" => "Light",
        "dark" => "Dark",
        _ => "System",
    }
}

fn next_theme(choice: &str) -> &'static str {
    match choice {
        "system" => "light",
        "light" => "dark",
        _ => "system",
    }
}

fn apply_saved_theme(manager: &StyleManager, button: &Button, icon: &Image) -> String {
    let theme = load_theme();
    configure_theme(manager, &theme);
    icon.set_icon_name(Some(theme_icon(&theme)));
    button.set_tooltip_text(Some(&format!(
        "Appearance: {} (click to change)",
        theme_label(&theme)
    )));
    theme
}
fn build_ui(app: &Application, present_on_start: bool) -> ApplicationWindow {
    let (control_tx, event_rx) = core::spawn_worker();
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Aether")
        .default_width(640)
        .default_height(760)
        .build();
    // Closing the window hides it; the worker and tunnel remain available to Quick Settings.
    window.connect_close_request(|window| {
        window.hide();
        gtk::glib::Propagation::Stop
    });

    let root = GtkBox::new(Orientation::Vertical, 0);
    let header = HeaderBar::new();
    let title = Label::new(Some("Aether"));
    title.add_css_class("title");
    header.set_title_widget(Some(&title));
    let theme_button = Button::new();
    let theme_icon_image = Image::from_icon_name(theme_icon(&load_theme()));
    theme_button.set_child(Some(&theme_icon_image));
    theme_button.set_tooltip_text(Some(&format!(
        "Appearance: {} - click to change",
        theme_label(&load_theme())
    )));
    theme_button.add_css_class("flat");
    header.pack_end(&theme_button);
    root.append(&header);

    let stack = Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_vexpand(true);
    let switcher = StackSwitcher::new();
    switcher.set_stack(Some(&stack));
    switcher.set_halign(Align::Center);
    switcher.set_margin_top(8);
    switcher.set_margin_bottom(8);
    root.append(&switcher);

    let connection_page = GtkBox::new(Orientation::Vertical, 0);
    connection_page.set_margin_top(12);
    connection_page.set_margin_bottom(10);
    connection_page.set_margin_start(12);
    connection_page.set_margin_end(12);
    connection_page.set_hexpand(true);
    connection_page.set_vexpand(true);

    let hero = GtkBox::new(Orientation::Vertical, 0);
    hero.set_halign(Align::Center);
    hero.set_valign(Align::Center);
    hero.set_vexpand(true);
    let connect = Button::new();
    connect.set_cursor_from_name(Some("pointer"));
    let connect_visuals = Stack::new();
    connect_visuals.set_transition_type(gtk::StackTransitionType::Crossfade);
    connect_visuals.set_transition_duration(100);
    connect_visuals.set_size_request(56, 56);
    let connect_icon = Image::from_icon_name("media-playback-start-symbolic");
    connect_icon.set_pixel_size(56);
    let connect_spinner = Spinner::new();
    connect_spinner.set_size_request(56, 56);
    connect_visuals.add_named(&connect_icon, Some("icon"));
    connect_visuals.add_named(&connect_spinner, Some("spinner"));
    connect.set_child(Some(&connect_visuals));
    connect.add_css_class("suggested-action");
    connect.add_css_class("connect-circle");
    set_connect_visual(
        &connect,
        &connect_visuals,
        &connect_icon,
        &connect_spinner,
        ConnectVisual::Disconnected,
    );
    hero.append(&connect);
    connection_page.append(&hero);

    let quick_profiles = GtkBox::new(Orientation::Horizontal, 8);
    quick_profiles.set_margin_top(10);
    quick_profiles.set_margin_bottom(8);
    quick_profiles.set_halign(Align::Center);
    let (mode_row, quick_mode) = quick_dropdown("Profile", MODES, 148);
    let (scan_row, quick_scan) = quick_dropdown("Scan", SCANS, 92);
    let (noize_row, quick_noize) = quick_dropdown("Obfuscation", NOIZE, 112);
    let (direction_row, quick_direction) = quick_dropdown("Chain", CHAIN_DIRECTIONS, 104);
    direction_row.set_visible(false);
    quick_profiles.append(&mode_row);
    quick_profiles.append(&scan_row);
    quick_profiles.append(&noize_row);
    quick_profiles.append(&direction_row);
    connection_page.append(&quick_profiles);
    let psiphon_setup = append_psiphon_setup(&connection_page);
    psiphon_setup.frame.set_visible(false);

    let footer_spacer = GtkBox::new(Orientation::Vertical, 0);
    footer_spacer.set_vexpand(true);
    connection_page.append(&footer_spacer);

    let proxy_line = GtkBox::new(Orientation::Horizontal, 6);
    proxy_line.set_halign(Align::Center);
    let proxy_label = Label::new(Some("SOCKS5 - 127.0.0.1:1819"));
    proxy_label.add_css_class("caption");
    proxy_label.add_css_class("dim-label");
    let copy_proxy = Button::from_icon_name("edit-copy-symbolic");
    copy_proxy.add_css_class("flat");
    copy_proxy.set_tooltip_text(Some("Copy SOCKS5 address"));
    copy_proxy.update_property(&[gtk::accessible::Property::Label("Copy SOCKS5 address")]);
    proxy_line.append(&proxy_label);
    proxy_line.append(&copy_proxy);
    connection_page.append(&proxy_line);
    add_page(&stack, "connect", "Connect", &connection_page);

    let settings_scroll = ScrolledWindow::new();
    settings_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    let settings_page = GtkBox::new(Orientation::Vertical, 12);
    settings_page.set_margin_top(20);
    settings_page.set_margin_bottom(24);
    settings_page.set_margin_start(24);
    settings_page.set_margin_end(24);
    let settings_intro = Label::new(Some("Credentials are only used for this connection."));
    settings_intro.set_wrap(true);
    settings_intro.set_halign(Align::Start);
    settings_intro.add_css_class("dim-label");
    settings_page.append(&settings_intro);
    let (autostart_switch, auto_connect_switch, autostart_status) =
        append_autostart_setting(&settings_page);
    let autostart_status_switch = autostart_status.clone();
    let auto_connect_startup_switch = auto_connect_switch.clone();
    let updating_autostart = Rc::new(Cell::new(false));
    let updating_autostart_switch = updating_autostart.clone();
    autostart_switch.connect_active_notify(move |switch| {
        if updating_autostart_switch.get() {
            return;
        }
        let enabled = switch.is_active();
        match set_autostart_enabled(enabled) {
            Ok(()) => {
                auto_connect_startup_switch.set_sensitive(enabled);
                autostart_status_switch.set_visible(false);
            }
            Err(error) => {
                updating_autostart_switch.set(true);
                switch.set_active(!enabled);
                updating_autostart_switch.set(false);
                autostart_status_switch.set_text(&format!("Could not change startup: {error}"));
                autostart_status_switch.set_visible(true);
            }
        }
    });
    let auto_connect_status = autostart_status.clone();
    let updating_auto_connect = Rc::new(Cell::new(false));
    let updating_auto_connect_switch = updating_auto_connect.clone();
    auto_connect_switch.connect_active_notify(move |switch| {
        if updating_auto_connect_switch.get() {
            return;
        }
        let enabled = switch.is_active();
        match set_auto_connect_enabled(enabled) {
            Ok(()) => auto_connect_status.set_visible(false),
            Err(error) => {
                updating_auto_connect_switch.set(true);
                switch.set_active(!enabled);
                updating_auto_connect_switch.set(false);
                auto_connect_status.set_text(&format!("Could not save auto-connect: {error}"));
                auto_connect_status.set_visible(true);
            }
        }
    });
    let sections = build_settings(&settings_page);
    let version = env!("CARGO_PKG_VERSION");
    let project_footer = Label::new(None);
    project_footer.set_markup(&format!(
        "Maintained by <a href=\"https://github.com/evokelektrique/AetherGTK\">evokelektrique</a> - <a href=\"https://github.com/evokelektrique/AetherGTK/releases/tag/v{version}\">v{version}</a>"
    ));
    project_footer.set_halign(Align::Center);
    project_footer.add_css_class("caption");
    project_footer.add_css_class("dim-label");
    settings_page.append(&project_footer);
    let sections = Rc::new(sections);
    let initial_profile = load_profile();
    let profile_widgets = Rc::new(vec![
        ("profile_mode", quick_mode.clone()),
        ("scan", quick_scan.clone()),
        ("noize", quick_noize.clone()),
        ("chain_direction", quick_direction.clone()),
        (
            "profile_mode",
            find_choice_dropdown(&sections, "profile_mode").unwrap(),
        ),
        ("scan", find_choice_dropdown(&sections, "scan").unwrap()),
        ("noize", find_choice_dropdown(&sections, "noize").unwrap()),
        (
            "quick_reconnect",
            find_choice_dropdown(&sections, "quick_reconnect").unwrap(),
        ),
        (
            "chain_direction",
            find_choice_dropdown(&sections, "chain_direction").unwrap(),
        ),
    ]);
    set_profile_widgets(initial_profile, &profile_widgets);
    let settings_state = Rc::new(RefCell::new(form_values(&sections)));
    let syncing_profile = Rc::new(Cell::new(false));
    refresh_visibility(&sections, &settings_state.borrow());
    let uses_warp = settings_state.borrow().profile.mode.uses_warp();
    scan_row.set_visible(uses_warp);
    noize_row.set_visible(uses_warp);
    direction_row.set_visible(matches!(
        initial_profile.mode,
        ProfileMode::TorWarp | ProfileMode::PsiphonWarp
    ));
    let direction_row_visibility = direction_row.clone();
    let scan_row_visibility = scan_row.clone();
    let noize_row_visibility = noize_row.clone();
    let psiphon_binary_entry = find_path_entry(&sections, "psiphon_bin");
    psiphon_setup.frame.set_visible(should_show_psiphon_setup(
        initial_profile.mode,
        psiphon_binary_entry.as_ref(),
    ));
    let prompt_binary_entry = psiphon_binary_entry.clone();
    let prompt_window = window.clone();
    let prompt_install = psiphon_setup.install.clone();
    let psiphon_frame = psiphon_setup.frame.clone();
    quick_mode.connect_selected_notify(move |dropdown| {
        let mode = MODES
            .get(dropdown.selected() as usize)
            .map(|choice| ProfileMode::parse(choice.value))
            .unwrap_or(ProfileMode::Masque);
        psiphon_frame.set_visible(should_show_psiphon_setup(
            mode,
            prompt_binary_entry.as_ref(),
        ));
        direction_row_visibility.set_visible(matches!(
            mode,
            ProfileMode::TorWarp | ProfileMode::PsiphonWarp
        ));
        scan_row_visibility.set_visible(mode.uses_warp());
        noize_row_visibility.set_visible(mode.uses_warp());
        maybe_prompt_psiphon_install(
            &prompt_window,
            mode,
            prompt_binary_entry.as_ref(),
            &prompt_install,
        );
    });
    settings_scroll.set_child(Some(&settings_page));
    add_page(&stack, "settings", "Settings", &settings_scroll);

    for (id, dropdown) in profile_widgets.iter() {
        let id = *id;
        let dropdown = dropdown.clone();
        let sections = sections.clone();
        let widgets = profile_widgets.clone();
        let state = settings_state.clone();
        let syncing = syncing_profile.clone();
        let psiphon_frame = psiphon_setup.frame.clone();
        let psiphon_binary_entry = psiphon_binary_entry.clone();
        dropdown.connect_selected_notify(move |source| {
            if id == "profile_mode" {
                let mode = MODES
                    .get(source.selected() as usize)
                    .map(|choice| ProfileMode::parse(choice.value))
                    .unwrap_or(ProfileMode::Masque);
                psiphon_frame.set_visible(should_show_psiphon_setup(
                    mode,
                    psiphon_binary_entry.as_ref(),
                ));
            }
            sync_profile_change(id, source, &sections, &widgets, &state, &syncing);
        });
    }

    for section in sections.iter() {
        for binding in &section.bindings {
            match &binding.input {
                Input::Choice(dropdown, _) => {
                    if matches!(
                        binding.spec.id,
                        "profile_mode" | "scan" | "noize" | "chain_direction" | "quick_reconnect"
                    ) {
                        continue;
                    }
                    let sections = sections.clone();
                    let state = settings_state.clone();
                    dropdown.connect_selected_notify(move |_| {
                        *state.borrow_mut() = form_values(&sections);
                        refresh_visibility(&sections, &state.borrow());
                    });
                }
                Input::Entry(entry) => {
                    let sections = sections.clone();
                    let state = settings_state.clone();
                    let visible_sections = sections.clone();
                    entry.connect_changed(move |_| {
                        *state.borrow_mut() = form_values(&visible_sections);
                        refresh_visibility(&sections, &state.borrow());
                    });
                }
                Input::Path(entry, browse) => {
                    let sections = sections.clone();
                    let state = settings_state.clone();
                    let visible_sections = sections.clone();
                    let psiphon_setup_frame = psiphon_setup.frame.clone();
                    let psiphon_binary_entry = entry.clone();
                    entry.connect_changed(move |_| {
                        *state.borrow_mut() = form_values(&visible_sections);
                        refresh_visibility(&sections, &state.borrow());
                        psiphon_setup_frame.set_visible(should_show_psiphon_setup(
                            state.borrow().profile.mode,
                            Some(&psiphon_binary_entry),
                        ));
                    });
                    let entry = entry.clone();
                    let window = window.clone();
                    let setup_status = psiphon_setup.status.clone();
                    browse.connect_clicked(move |_| {
                        choose_psiphon_client(&window, &entry, &setup_status);
                    });
                }
                Input::Switch(switch) => {
                    let sections = sections.clone();
                    let state = settings_state.clone();
                    switch.connect_active_notify(move |_| {
                        *state.borrow_mut() = form_values(&sections);
                        refresh_visibility(&sections, &state.borrow());
                    });
                }
            }
        }
    }

    let activity_page = GtkBox::new(Orientation::Vertical, 10);
    activity_page.set_margin_top(18);
    activity_page.set_margin_bottom(18);
    activity_page.set_margin_start(20);
    activity_page.set_margin_end(20);
    let activity_toolbar = GtkBox::new(Orientation::Horizontal, 8);
    let activity_heading = Label::new(Some("Activity"));
    activity_heading.add_css_class("title-2");
    activity_heading.set_halign(Align::Start);
    activity_heading.set_hexpand(true);
    let wrap_logs = ToggleButton::new();
    wrap_logs.set_child(Some(&Image::from_icon_name("view-wrapped-symbolic")));
    wrap_logs.set_active(false);
    wrap_logs.add_css_class("flat");
    wrap_logs.set_tooltip_text(Some("Enable line wrapping"));
    wrap_logs.update_property(&[gtk::accessible::Property::Label("Toggle line wrapping")]);
    let logs = TextView::new();
    logs.set_editable(false);
    logs.set_cursor_visible(false);
    logs.set_monospace(true);
    logs.set_wrap_mode(gtk::WrapMode::None);
    logs.set_left_margin(18);
    logs.set_right_margin(18);
    logs.set_top_margin(16);
    logs.set_bottom_margin(16);
    logs.set_pixels_above_lines(2);
    logs.set_pixels_below_lines(2);
    logs.buffer().set_text("Aether events will appear here.\n");
    let scroll_logs = logs.clone();
    logs.buffer().connect_changed(move |buffer| {
        let mut end = buffer.end_iter();
        scroll_logs.scroll_to_iter(&mut end, 0.0, false, 0.0, 1.0);
    });
    let activity_tags = activity_tags(&logs.buffer());
    let copy_logs = Button::from_icon_name("edit-copy-symbolic");
    copy_logs.add_css_class("flat");
    copy_logs.set_tooltip_text(Some("Copy activity"));
    copy_logs.update_property(&[gtk::accessible::Property::Label("Copy activity")]);
    let log_buffer = logs.buffer();
    let copy_buffer = log_buffer.clone();
    let copy_logs_tooltip = copy_logs.clone();
    copy_logs.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let text = copy_buffer.text(&copy_buffer.start_iter(), &copy_buffer.end_iter(), false);
            display.clipboard().set_text(&text);
            show_copied_tooltip(&copy_logs_tooltip, "Copy activity");
        }
    });
    activity_toolbar.append(&activity_heading);
    activity_toolbar.append(&copy_logs);
    activity_toolbar.append(&wrap_logs);
    let clear_logs = Button::with_label("Clear activity");
    clear_logs.add_css_class("flat");
    clear_logs.set_halign(Align::End);
    clear_logs.set_tooltip_text(Some("Clear activity log"));
    clear_logs.connect_clicked(move |_| log_buffer.set_text(""));
    let logs_scroll = ScrolledWindow::new();
    logs_scroll.set_vexpand(true);
    logs_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    logs_scroll.add_css_class("card");
    logs_scroll.add_css_class("activity-log-frame");
    logs_scroll.set_overflow(gtk::Overflow::Hidden);
    logs_scroll.set_child(Some(&logs));
    let wrapped_logs = logs.clone();
    let wrapped_scroll = logs_scroll.clone();
    wrap_logs.connect_toggled(move |button| {
        let wrapped = button.is_active();
        wrapped_logs.set_wrap_mode(if wrapped {
            gtk::WrapMode::WordChar
        } else {
            gtk::WrapMode::None
        });
        wrapped_scroll.set_policy(
            if wrapped {
                gtk::PolicyType::Never
            } else {
                gtk::PolicyType::Automatic
            },
            gtk::PolicyType::Automatic,
        );
        button.set_tooltip_text(Some(if wrapped {
            "Disable line wrapping"
        } else {
            "Enable line wrapping"
        }));
    });
    activity_page.append(&activity_toolbar);
    activity_page.append(&logs_scroll);
    activity_page.append(&clear_logs);
    add_page(&stack, "activity", "Activity", &activity_page);

    root.append(&stack);
    window.set_content(Some(&root));

    let manager = StyleManager::default();
    set_activity_tag_colors(manager.is_dark(), &activity_tags);
    let activity_theme_tags = activity_tags.clone();
    manager.connect_dark_notify(move |manager| {
        set_activity_tag_colors(manager.is_dark(), &activity_theme_tags);
    });
    let current_theme = Rc::new(RefCell::new(apply_saved_theme(
        &manager,
        &theme_button,
        &theme_icon_image,
    )));
    if let Some(display) = gtk::gdk::Display::default() {
        let css = gtk::CssProvider::new();
        css.load_from_data(
            ".activity-log-frame { border-radius: 12px; } .connect-circle { min-width: 148px; min-height: 148px; border-radius: 9999px; padding: 0; } .connect-circle.connected { background-color: #26a269; background-image: none; color: #ffffff; } .connect-circle.connected:hover { background-color: #2ec27e; } .connect-circle.connected:active { background-color: #208a5e; }",
        );
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    let manager = manager.clone();
    let current_theme = current_theme.clone();
    theme_button.connect_clicked(move |button| {
        let mut choice = current_theme.borrow_mut();
        *choice = next_theme(&choice).into();
        configure_theme(&manager, &choice);
        save_theme(&choice);
        theme_icon_image.set_icon_name(Some(theme_icon(&choice)));
        button.set_tooltip_text(Some(&format!(
            "Appearance: {} - click to change",
            theme_label(&choice)
        )));
    });

    let display = gtk::gdk::Display::default();
    let proxy_label_copy = proxy_label.clone();
    let copy_proxy_tooltip = copy_proxy.clone();
    copy_proxy.connect_clicked(move |_| {
        if let Some(display) = &display {
            display
                .clipboard()
                .set_text(&proxy_label_copy.text().replace("SOCKS5  -  ", ""));
            show_copied_tooltip(&copy_proxy_tooltip, "Copy SOCKS5 address");
        }
    });

    let (install_tx, install_rx) = std::sync::mpsc::channel();
    let install_status = psiphon_setup.status.clone();
    let install_button = psiphon_setup.install.clone();
    let install_progress = psiphon_setup.progress.clone();
    psiphon_setup.install.connect_clicked(move |_| {
        install_button.set_sensitive(false);
        install_progress.set_fraction(0.0);
        install_progress.set_visible(true);
        install_status.set_text("Building Psiphon client. This may take a few minutes.");
        let sender = install_tx.clone();
        let result = std::thread::Builder::new()
            .name("psiphon-installer".into())
            .spawn(move || {
                let _ = sender.send(psiphon_setup::install_client());
            });
        if let Err(error) = result {
            install_status.set_text(&format!("Could not start the builder: {error}"));
            install_progress.set_visible(false);
            install_button.set_sensitive(true);
            install_button.set_tooltip_text(Some("Retry Psiphon installation"));
        }
    });

    let tx = control_tx.clone();
    let connect_icon_state = connect_icon.clone();
    let connect_visuals_state = connect_visuals.clone();
    let connect_spinner_state = connect_spinner.clone();
    let state = settings_state.clone();
    let log_view = logs.clone();
    let connect_activity_tags = activity_tags.clone();
    let connect_window = window.clone();
    let connect_psiphon_entry = find_path_entry(&sections, "psiphon_bin");
    let connect_install_button = psiphon_setup.install.clone();
    let quick_profiles_active = quick_profiles.clone();
    let settings_page_active = settings_page.clone();
    let tunnel_active = Rc::new(Cell::new(false));
    let panel_status = Rc::new(RefCell::new("Disconnected".to_owned()));
    let panel_proxy_address = Rc::new(RefCell::new(String::new()));
    let active = tunnel_active.clone();
    let panel_status_click = panel_status.clone();
    connect.connect_clicked(move |button| {
        if active.get() {
            *panel_status_click.borrow_mut() = "Disconnecting".into();
            let _ = tx.send(core::Control::Stop);
            set_connect_visual(
                button,
                &connect_visuals_state,
                &connect_icon_state,
                &connect_spinner_state,
                ConnectVisual::Disconnecting,
            );
            button.set_sensitive(false);
            return;
        }
        let state = state.borrow();
        if matches!(
            state.profile.mode,
            ProfileMode::PsiphonOnly | ProfileMode::PsiphonWarp
        ) {
            let explicit = state
                .values
                .get("psiphon_bin")
                .filter(|path| !path.is_empty())
                .map(|path| PathBuf::from(path.as_str()));
            if !psiphon_setup::resolve_client_path(
                explicit.as_deref(),
                &psiphon_setup::managed_client_path(),
            )
            .is_ok_and(|client| client.is_some())
            {
                maybe_prompt_psiphon_install(
                    &connect_window,
                    state.profile.mode,
                    connect_psiphon_entry.as_ref(),
                    &connect_install_button,
                );
                return;
            }
        }
        if state.profile.mode == ProfileMode::ZeroTrust
            && state.values.get("team").is_none_or(String::is_empty)
        {
            button.set_tooltip_text(Some("Enter your team name in Settings before connecting"));
            return;
        }
        if state
            .values
            .get("psiphon_mode")
            .is_some_and(|mode| mode != "off")
        {
            if let Some(path) = state
                .values
                .get("psiphon_bin")
                .filter(|path| !path.is_empty())
            {
                if !psiphon_setup::is_executable_client(Path::new(path)) {
                    let message =
                        format!("Selected Psiphon client is missing or not executable: {path}");
                    button.set_tooltip_text(Some(&message));
                    append_activity(
                        &log_view.buffer(),
                        &format!("ERROR {message}\n"),
                        &connect_activity_tags,
                    );
                    return;
                }
            }
        }
        let _ = tx.send(core::Control::Start(
            state.args.clone(),
            state.environment.clone(),
            state.log_level.clone(),
        ));
        active.set(true);
        *panel_status_click.borrow_mut() = "Connecting".into();
        set_connect_visual(
            button,
            &connect_visuals_state,
            &connect_icon_state,
            &connect_spinner_state,
            ConnectVisual::Connecting,
        );
        button.set_sensitive(true);
        quick_profiles_active.set_sensitive(false);
        settings_page_active.set_sensitive(false);
    });

    let connection = app
        .dbus_connection()
        .expect("GTK application should be registered on the session bus");
    let introspection = gio::DBusNodeInfo::for_xml(include_str!(
        "../data/io.github.evokelektrique.AetherGtk.Control.xml"
    ))
    .expect("Aether D-Bus introspection XML should be valid");
    let interface = introspection
        .lookup_interface("io.github.evokelektrique.AetherGtk.Control")
        .expect("Aether D-Bus control interface should exist");
    let state_for_dbus = settings_state.clone();
    let status_for_dbus = panel_status.clone();
    let proxy_for_dbus = panel_proxy_address.clone();
    let active_for_dbus = tunnel_active.clone();
    let connect_for_dbus = connect.clone();
    let window_for_dbus = window.clone();
    let app_for_dbus = app.clone();
    let quit_after_disconnect = Rc::new(Cell::new(false));
    let quit_after_disconnect_dbus = quit_after_disconnect.clone();
    let _registration_id = connection
        .register_object("/io/github/evokelektrique/AetherGtk/Control", &interface)
        .method_call(move |_, _, _, _, method, _, invocation| match method {
            "GetState" => {
                let status = status_for_dbus.borrow().clone();
                let profile = profile_label(state_for_dbus.borrow().profile.mode).to_owned();
                let address = if status == "Connected" {
                    proxy_for_dbus.borrow().clone()
                } else {
                    String::new()
                };
                invocation.return_value(Some(&(status, profile, address).to_variant()));
            }
            "Connect" => {
                if !active_for_dbus.get() {
                    connect_for_dbus.emit_clicked();
                }
                invocation.return_value(Some(&().to_variant()));
            }
            "Disconnect" => {
                if active_for_dbus.get() {
                    connect_for_dbus.emit_clicked();
                }
                invocation.return_value(Some(&().to_variant()));
            }
            "ShowWindow" => {
                window_for_dbus.present();
                invocation.return_value(Some(&().to_variant()));
            }
            "Quit" => {
                if active_for_dbus.get() {
                    quit_after_disconnect_dbus.set(true);
                    connect_for_dbus.emit_clicked();
                } else {
                    app_for_dbus.quit();
                }
                invocation.return_value(Some(&().to_variant()));
            }
            _ => invocation.return_dbus_error(
                "io.github.evokelektrique.AetherGtk.Control.UnknownMethod",
                "Unknown Aether control method",
            ),
        })
        .build()
        .expect("Aether D-Bus control interface should register");
    let _control_name_owner = gio::bus_own_name_on_connection(
        &connection,
        "io.github.evokelektrique.AetherGtk.Control",
        gio::BusNameOwnerFlags::NONE,
        |_, _| {},
        |_, _| {},
    );

    let connect_button = connect.clone();
    let connect_icon_state = connect_icon.clone();
    let connect_visuals_state = connect_visuals.clone();
    let connect_spinner_state = connect_spinner.clone();
    let active = tunnel_active.clone();
    let quick_profiles_state = quick_profiles.clone();
    let settings_page_state = settings_page.clone();
    let proxy_label = proxy_label.clone();
    let install_status = psiphon_setup.status.clone();
    let install_button = psiphon_setup.install.clone();
    let install_progress = psiphon_setup.progress.clone();
    let event_activity_tags = activity_tags.clone();
    let panel_status_events = panel_status.clone();
    let panel_proxy_events = panel_proxy_address.clone();
    let quit_after_disconnect_events = quit_after_disconnect.clone();
    let app_events = app.clone();
    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        while let Ok(result) = install_rx.try_recv() {
            install_progress.set_visible(false);
            match result {
                Ok(path) => {
                    install_button.set_sensitive(false);
                    install_button.set_tooltip_text(Some("Psiphon client is installed"));
                    psiphon_setup.frame.set_visible(false);
                    if let Some(entry) = &psiphon_binary_entry {
                        let current = entry.text();
                        let managed = psiphon_setup::managed_client_path();
                        if current.is_empty()
                            || Path::new(current.as_str()) == managed
                            || !psiphon_setup::is_executable_client(Path::new(current.as_str()))
                        {
                            entry.set_text(&path.to_string_lossy());
                        }
                    }
                    install_status.set_text(&format!("Installed at {}", path.display()));
                    append_activity(
                        &logs.buffer(),
                        &format!("Psiphon client installed at {}\n", path.display()),
                        &event_activity_tags,
                    );
                }
                Err(error) => {
                    install_button.set_sensitive(true);
                    install_button.set_tooltip_text(Some("Retry Psiphon installation"));
                    install_status.set_text(&error);
                    append_activity(
                        &logs.buffer(),
                        &format!("ERROR {error}\n"),
                        &event_activity_tags,
                    );
                }
            }
        }
        if !install_button.is_sensitive() {
            install_progress.pulse();
        }
        loop {
            // Worker events, rather than the button click, define connection state and proxy readiness.
            match event_rx.try_recv() {
                Ok(core::Event::Log(line)) => {
                    append_activity(&logs.buffer(), &format!("{line}\n"), &event_activity_tags);
                    let lower = line.to_lowercase();
                    if lower.contains("socks5 server listening on")
                        || lower.contains("psiphon is ready;")
                    {
                        let address = proxy_address_from_log(&line);
                        *panel_proxy_events.borrow_mut() = address.clone();
                        proxy_label.set_text(&format!("SOCKS5  -  {address}"));
                    }
                }
                Ok(core::Event::State(core::State::Starting)) => {
                    *panel_status_events.borrow_mut() = "Connecting".into();
                    set_connect_visual(
                        &connect_button,
                        &connect_visuals_state,
                        &connect_icon_state,
                        &connect_spinner_state,
                        ConnectVisual::Connecting,
                    );
                    connect_button.set_sensitive(true);
                }
                Ok(core::Event::State(core::State::Connected)) => {
                    *panel_status_events.borrow_mut() = "Connected".into();
                    set_connect_visual(
                        &connect_button,
                        &connect_visuals_state,
                        &connect_icon_state,
                        &connect_spinner_state,
                        ConnectVisual::Connected,
                    );
                    connect_button.set_sensitive(true);
                }
                Ok(core::Event::State(core::State::Stopping)) => {
                    *panel_status_events.borrow_mut() = "Disconnecting".into();
                    set_connect_visual(
                        &connect_button,
                        &connect_visuals_state,
                        &connect_icon_state,
                        &connect_spinner_state,
                        ConnectVisual::Disconnecting,
                    );
                    connect_button.set_sensitive(false);
                }
                Ok(core::Event::State(core::State::Disconnected)) => {
                    active.set(false);
                    *panel_status_events.borrow_mut() = "Disconnected".into();
                    set_connect_visual(
                        &connect_button,
                        &connect_visuals_state,
                        &connect_icon_state,
                        &connect_spinner_state,
                        ConnectVisual::Disconnected,
                    );
                    connect_button.set_sensitive(true);
                    quick_profiles_state.set_sensitive(true);
                    settings_page_state.set_sensitive(true);
                    if quit_after_disconnect_events.get() {
                        app_events.quit();
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return gtk::glib::ControlFlow::Break,
            }
        }
        gtk::glib::ControlFlow::Continue
    });

    if present_on_start {
        window.present();
        let psiphon_binary_entry = find_path_entry(&sections, "psiphon_bin");
        maybe_prompt_psiphon_install(
            &window,
            initial_profile.mode,
            psiphon_binary_entry.as_ref(),
            &psiphon_setup.install,
        );
    } else if autostart_path().is_file() && auto_connect_enabled() {
        connect.emit_clicked();
    }
    window
}
pub(super) fn run() {
    // D-Bus activation and autostart use this flag to start without presenting the window.
    let start_hidden = std::env::var_os("AETHER_GTK_BACKGROUND").is_some();
    let app = Application::builder()
        .application_id("io.github.evokelektrique.AetherGtk")
        .build();
    let window = Rc::new(RefCell::new(None::<ApplicationWindow>));
    let startup_window = window.clone();
    app.connect_startup(move |app| {
        *startup_window.borrow_mut() = Some(build_ui(app, !start_hidden));
    });
    let first_activation = Rc::new(Cell::new(true));
    let first_activation_signal = first_activation.clone();
    app.connect_activate(move |_| {
        if first_activation_signal.replace(false) {
            return;
        }
        if let Some(window) = window.borrow().as_ref() {
            window.present();
        }
    });
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_modes_map_and_persist() {
        assert_eq!(theme_scheme("system"), adw::ColorScheme::Default);
        assert_eq!(theme_scheme("light"), adw::ColorScheme::ForceLight);
        assert_eq!(theme_scheme("dark"), adw::ColorScheme::ForceDark);
        assert_eq!(next_theme("system"), "light");
        assert_eq!(next_theme("light"), "dark");
        assert_eq!(next_theme("dark"), "system");

        let path = std::env::temp_dir().join(format!(
            "aether-gtk-theme-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for choice in ["system", "light", "dark"] {
            write_theme(&path, choice);
            assert_eq!(read_theme(&path), choice);
        }
        let _ = fs::remove_file(path);
    }
}
