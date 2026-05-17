use nu_table::string_truncate;

pub use nu_table::string_width;

pub fn truncate_str(text: &mut String, width: usize) {
    if width == 0 {
        text.clear();
    } else {
        if string_width(text) < width {
            return;
        }

        *text = string_truncate(text, width - 1);
        text.push('…');
    }
}
