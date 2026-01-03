use tower_lsp::lsp_types::{Position, Range};

/// Convert LSP position (UTF-16 code units) to byte offset in text
///
/// LSP uses UTF-16 code units for character offsets, but Rust strings use UTF-8 bytes.
/// This function handles the conversion, including multi-byte characters like emojis.
pub fn lsp_position_to_byte_offset(text: &str, position: Position) -> usize {
    let mut current_line = 0;
    let mut byte_offset = 0;

    for line in text.lines() {
        if current_line == position.line as usize {
            // Convert UTF-16 offset to byte offset within line
            let utf16_offset = position.character as usize;
            let mut utf16_count = 0;

            for (byte_idx, ch) in line.char_indices() {
                if utf16_count >= utf16_offset {
                    return byte_offset + byte_idx;
                }
                utf16_count += ch.len_utf16();
            }

            // Position is at end of line
            return byte_offset + line.len();
        }

        byte_offset += line.len() + 1; // +1 for newline
        current_line += 1;
    }

    // Position is beyond end of file
    byte_offset
}

/// Convert byte offset to LSP position (UTF-16 code units)
pub fn byte_offset_to_lsp_position(text: &str, offset: usize) -> Position {
    let mut current_line = 0;
    let mut line_start_byte = 0;

    for line in text.lines() {
        let line_end_byte = line_start_byte + line.len();

        if offset <= line_end_byte {
            // Offset is on this line
            let byte_in_line = offset - line_start_byte;

            // Convert byte offset to UTF-16 offset
            let mut utf16_count = 0;
            let mut current_byte = 0;

            for ch in line.chars() {
                if current_byte >= byte_in_line {
                    break;
                }
                utf16_count += ch.len_utf16();
                current_byte += ch.len_utf8();
            }

            return Position {
                line: current_line,
                character: utf16_count as u32,
            };
        }

        line_start_byte = line_end_byte + 1; // +1 for newline
        current_line += 1;
    }

    // Offset is beyond end of file
    Position {
        line: current_line,
        character: 0,
    }
}

/// Convert a byte span to LSP range
pub fn byte_span_to_lsp_range(text: &str, start: usize, end: usize) -> Range {
    Range {
        start: byte_offset_to_lsp_position(text, start),
        end: byte_offset_to_lsp_position(text, end),
    }
}

#[cfg(test)]
#[path = "position/position_tests.rs"]
mod position_tests;
