//! Connection controls and activity-log presentation.

use crate::profile::*;
use crate::settings::Choice;
use adw::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, DropDown, Image, Label, Orientation, Spinner, Stack, StringList,
};
use std::time::Duration;

#[derive(Clone)]
pub(super) struct ActivityTags {
    pub(super) info: gtk::TextTag,
    pub(super) warn: gtk::TextTag,
    pub(super) error: gtk::TextTag,
}
pub(super) fn quick_dropdown(
    label: &str,
    choices: &'static [Choice],
    width: i32,
) -> (GtkBox, DropDown) {
    let row = GtkBox::new(Orientation::Vertical, 4);
    let title = Label::new(Some(label));
    title.set_halign(Align::Start);
    title.add_css_class("caption");
    title.add_css_class("dim-label");
    let labels: Vec<_> = choices.iter().map(|choice| choice.label).collect();
    let dropdown = DropDown::new(Some(StringList::new(&labels)), gtk::Expression::NONE);
    dropdown.set_size_request(width, -1);
    dropdown.set_hexpand(false);
    dropdown.set_tooltip_text(Some(label));
    row.append(&title);
    row.append(&dropdown);
    row.set_widget_name(&format!("profile-control-{label}"));
    (row, dropdown)
}

#[derive(Clone, Copy)]
pub(super) enum ConnectVisual {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

pub(super) fn set_connect_visual(
    button: &Button,
    visuals: &Stack,
    icon: &Image,
    spinner: &Spinner,
    visual: ConnectVisual,
) {
    let (icon_name, label) = connect_visual(visual);
    icon.set_icon_name(Some(icon_name));
    if matches!(
        visual,
        ConnectVisual::Connecting | ConnectVisual::Disconnecting
    ) {
        spinner.start();
        visuals.set_visible_child_name("spinner");
    } else {
        spinner.stop();
        visuals.set_visible_child_name("icon");
    }
    button.remove_css_class("connected");
    if matches!(visual, ConnectVisual::Connected) {
        button.add_css_class("connected");
    }
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
}

pub(super) fn show_copied_tooltip(button: &Button, original: &'static str) {
    button.set_tooltip_text(Some("Copied"));
    button.trigger_tooltip_query();
    let button = button.clone();
    gtk::glib::timeout_add_local_once(Duration::from_secs(2), move || {
        button.set_tooltip_text(Some(original));
    });
}

pub(super) fn connect_visual(visual: ConnectVisual) -> (&'static str, &'static str) {
    match visual {
        ConnectVisual::Disconnected => ("media-playback-start-symbolic", "Connect to Aether"),
        ConnectVisual::Connecting => ("process-working-symbolic", "Cancel connection"),
        ConnectVisual::Connected => ("media-playback-stop-symbolic", "Disconnect from Aether"),
        ConnectVisual::Disconnecting => ("process-working-symbolic", "Disconnecting"),
    }
}

pub(super) fn profile_label(mode: ProfileMode) -> &'static str {
    MODES
        .iter()
        .find(|choice| choice.value == mode.value())
        .map(|choice| choice.label)
        .unwrap_or("MASQUE - modern")
}
pub(super) fn activity_tags(buffer: &gtk::TextBuffer) -> ActivityTags {
    let tag = |name| {
        buffer
            .create_tag(Some(name), &[])
            .expect("activity text tag should be unique")
    };
    ActivityTags {
        info: tag("activity-info"),
        warn: tag("activity-warn"),
        error: tag("activity-error"),
    }
}

pub(super) fn set_activity_tag_colors(dark: bool, tags: &ActivityTags) {
    tags.info
        .set_foreground(Some(if dark { "#78aeed" } else { "#1c71d8" }));
    tags.warn
        .set_foreground(Some(if dark { "#f6d32d" } else { "#a34700" }));
    tags.error
        .set_foreground(Some(if dark { "#ff7b63" } else { "#c01c28" }));
}

pub(super) fn append_activity(buffer: &gtk::TextBuffer, message: &str, tags: &ActivityTags) {
    // Apply color only to the level prefix so the original log remains intact for copying.
    let tag = if message.starts_with("INFO ") {
        Some(&tags.info)
    } else if message.starts_with("WARN ") {
        Some(&tags.warn)
    } else if message.starts_with("ERROR ") {
        Some(&tags.error)
    } else {
        None
    };
    let mut end = buffer.end_iter();
    if let Some(tag) = tag {
        let prefix_len = message.find(char::is_whitespace).unwrap_or(message.len());
        buffer.insert_with_tags(&mut end, &message[..prefix_len], &[tag]);
        buffer.insert(&mut end, &message[prefix_len..]);
    } else {
        buffer.insert(&mut end, message);
    }
}
pub(super) fn add_page(stack: &Stack, name: &str, title: &str, content: &impl IsA<gtk::Widget>) {
    stack.add_titled(content, Some(name), title);
}
pub(super) fn proxy_address_from_log(line: &str) -> String {
    line.rsplit_once("listening on")
        .map(|(_, address)| address.trim().to_owned())
        .or_else(|| {
            line.to_lowercase()
                .split_once("psiphon is ready;")
                .and_then(|(_, address)| address.split_once(" leaves").map(|(addr, _)| addr.trim()))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "127.0.0.1:1819".into())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_button_uses_actionable_icons_and_accessible_labels() {
        assert_eq!(
            connect_visual(ConnectVisual::Disconnected),
            ("media-playback-start-symbolic", "Connect to Aether")
        );
        assert_eq!(
            connect_visual(ConnectVisual::Connecting),
            ("process-working-symbolic", "Cancel connection")
        );
        assert_eq!(
            connect_visual(ConnectVisual::Connected),
            ("media-playback-stop-symbolic", "Disconnect from Aether")
        );
        assert_eq!(
            connect_visual(ConnectVisual::Disconnecting),
            ("process-working-symbolic", "Disconnecting")
        );
    }
}
