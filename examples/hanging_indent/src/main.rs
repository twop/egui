#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{
    self, FontId,
    text::{ByteIndex, LayoutJob, LayoutSection, TextFormat},
};

const INITIAL_TEXT: &str = "\
Shopping list:
- milk, eggs, bread, and whatever else we need for the weekend, which is a long list
- things for the garden
  - seeds for tomatoes, cucumbers, peppers, and some herbs if they have any left
  - a new hose, since the old one has a leak somewhere in the middle
continuation after a hard break, aligned with the text above, which also wraps
1. numbered items work too, and wrap just like the bullet points do

A normal paragraph is not indented, even when it is long enough to wrap onto the next row.";

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 480.0]),
        ..Default::default()
    };

    let mut text = INITIAL_TEXT.to_owned();
    let mut width = 360.0;
    let mut enabled = true;

    eframe::run_ui_native("Hanging indent", options, move |ui, _frame| {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add(egui::Slider::new(&mut width, 100.0..=500.0).text("width"));
            ui.checkbox(&mut enabled, "Hanging indent");
            ui.separator();

            let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let job = list_layout_job(ui, buf.as_str(), wrap_width, enabled);
                ui.fonts_mut(|f| f.layout_job(job))
            };
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_width(width)
                    .layouter(&mut layouter),
            );
        });
    })
}

/// One section per line (two for list items: the marker and the rest).
///
/// List items get a hanging indent equal to the width of their prefix (leading spaces + marker),
/// so wrapped rows line up with the text after the marker.
/// A non-empty line after a list item continues it, and is indented the same way.
/// An empty line ends the list item, but is still indented, since that is where the text goes
/// if you type a continuation (e.g. after pressing enter at the end of a list item).
fn list_layout_job(ui: &egui::Ui, text: &str, wrap_width: f32, enabled: bool) -> LayoutJob {
    let font_id = FontId::proportional(16.0);
    let text_format = TextFormat::simple(font_id.clone(), ui.visuals().text_color());
    let marker_format = TextFormat::simple(font_id, ui.visuals().weak_text_color());

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    // The indent of the list item we are in, if any:
    let mut item_indent = None;

    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches('\n');

        if let Some(prefix_len) = list_prefix_len(content) {
            let (prefix, rest) = line.split_at(prefix_len);
            let indent = if enabled {
                text_width(ui, prefix, &marker_format)
            } else {
                0.0
            };
            // The hanging indent is taken from the section with the first character of the line:
            push_section(&mut job, prefix, 0.0, indent, marker_format.clone());
            push_section(&mut job, rest, 0.0, 0.0, text_format.clone());
            item_indent = Some(indent);
        } else if let Some(indent) = item_indent {
            // Continuation of a list item after a hard break: indent all of its rows.
            // Any leading spaces in the text count towards the indent.
            let num_spaces = content.len() - content.trim_start().len();
            let spaces_width = text_width(ui, &content[..num_spaces], &text_format);
            let leading_space = (indent - spaces_width).max(0.0);
            push_section(&mut job, line, leading_space, indent, text_format.clone());
            if content.trim().is_empty() {
                item_indent = None; // An empty line ends the list item.
            }
        } else {
            push_section(&mut job, line, 0.0, 0.0, text_format.clone());
        }
    }

    if let Some(indent) = item_indent
        && text.ends_with('\n')
    {
        // The text ends with an empty line after a list item, e.g. after pressing enter at the end.
        // An empty section is enough to put the cursor at the indent:
        push_section(&mut job, "", indent, indent, text_format);
    }

    job
}

/// Length in bytes of the list prefix (leading spaces, marker, and a space), if this is a list item.
///
/// Handles `- `, `* `, `+ `, and `1. `.
fn list_prefix_len(line: &str) -> Option<usize> {
    let rest = line.trim_start_matches(' ');
    let num_spaces = line.len() - rest.len();

    let marker_len = if rest.starts_with(['-', '*', '+']) {
        1
    } else {
        let num_digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if 0 < num_digits && rest[num_digits..].starts_with('.') {
            num_digits + 1
        } else {
            return None;
        }
    };

    rest[marker_len..]
        .starts_with(' ')
        .then_some(num_spaces + marker_len + 1)
}

/// The width of `text` laid out on a single row.
fn text_width(ui: &egui::Ui, text: &str, format: &TextFormat) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let mut job = LayoutJob::single_section(text.to_owned(), format.clone());
    job.round_output_to_gui = false; // We want the exact width, so the indent matches the text.
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    galley.rows.first().map_or(0.0, |row| row.size.x)
}

fn push_section(
    job: &mut LayoutJob,
    text: &str,
    leading_space: f32,
    hanging_indent: f32,
    format: TextFormat,
) {
    // We don't use `LayoutJob::append`, since it may merge this into the previous section,
    // but we need each line to start a new section with its own indent.
    let start = job.text.len();
    job.text += text;
    job.sections.push(LayoutSection {
        leading_space,
        hanging_indent,
        byte_range: ByteIndex(start)..ByteIndex(job.text.len()),
        format,
    });
}

#[test]
fn test_list_prefix_len() {
    assert_eq!(list_prefix_len("- item"), Some(2));
    assert_eq!(list_prefix_len("  * item"), Some(4));
    assert_eq!(list_prefix_len("12. item"), Some(4));
    assert_eq!(list_prefix_len("-item"), None);
    assert_eq!(list_prefix_len("1.item"), None);
    assert_eq!(list_prefix_len("plain text"), None);
    assert_eq!(list_prefix_len(""), None);
}
