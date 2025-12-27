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
mod tests {
    use super::*;

    #[test]
    fn test_ascii_text() {
        let text = "Hello\nWorld";

        // Position (0, 0) -> byte 0
        assert_eq!(lsp_position_to_byte_offset(text, Position { line: 0, character: 0 }), 0);

        // Position (0, 5) -> byte 5 (end of "Hello")
        assert_eq!(lsp_position_to_byte_offset(text, Position { line: 0, character: 5 }), 5);

        // Position (1, 0) -> byte 6 (start of "World")
        assert_eq!(lsp_position_to_byte_offset(text, Position { line: 1, character: 0 }), 6);

        // Position (1, 5) -> byte 11 (end of "World")
        assert_eq!(lsp_position_to_byte_offset(text, Position { line: 1, character: 5 }), 11);
    }

    #[test]
    fn test_emoji() {
        let text = "Hello 😀 World";

        // "😀" is 4 bytes (F0 9F 98 80) but 2 UTF-16 code units
        // Position (0, 7) should be after emoji (byte 11)
        assert_eq!(lsp_position_to_byte_offset(text, Position { line: 0, character: 8 }), 11);
    }

    #[test]
    fn test_byte_to_position() {
        let text = "Hello\nWorld";

        assert_eq!(byte_offset_to_lsp_position(text, 0), Position { line: 0, character: 0 });
        assert_eq!(byte_offset_to_lsp_position(text, 5), Position { line: 0, character: 5 });
        assert_eq!(byte_offset_to_lsp_position(text, 6), Position { line: 1, character: 0 });
        assert_eq!(byte_offset_to_lsp_position(text, 11), Position { line: 1, character: 5 });
    }

    #[test]
    fn test_byte_to_position_emoji() {
        let text = "Hi 😀";

        // "😀" starts at byte 3, ends at byte 7
        // Position before emoji: (0, 3)
        assert_eq!(byte_offset_to_lsp_position(text, 3), Position { line: 0, character: 3 });

        // Position after emoji: (0, 5) because emoji is 2 UTF-16 units
        assert_eq!(byte_offset_to_lsp_position(text, 7), Position { line: 0, character: 5 });
    }

    #[test]
    fn test_span_to_range() {
        let text = "User {\n  name: string\n}";

        // Span of "User" (bytes 0-4)
        let range = byte_span_to_lsp_range(text, 0, 4);
        assert_eq!(range.start, Position { line: 0, character: 0 });
        assert_eq!(range.end, Position { line: 0, character: 4 });

        // Span of "name" (bytes 10-14)
        let range = byte_span_to_lsp_range(text, 10, 14);
        assert_eq!(range.start, Position { line: 1, character: 2 });
        assert_eq!(range.end, Position { line: 1, character: 6 });
    }
}
