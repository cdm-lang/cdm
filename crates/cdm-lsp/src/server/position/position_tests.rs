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
    // UTF-16: H(0) e(1) l(2) l(3) o(4) sp(5) 😀(6-7) sp(8) W(9)...
    // Bytes:  H(0) e(1) l(2) l(3) o(4) sp(5) 😀(6-9) sp(10) W(11)...
    // Position (0, 8) should be the space after emoji (byte 10)
    assert_eq!(lsp_position_to_byte_offset(text, Position { line: 0, character: 8 }), 10);
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

    // Span of "name" (bytes 9-13)
    // Line 1: "  name: string"
    // Bytes: sp(7) sp(8) n(9) a(10) m(11) e(12) :(13)
    let range = byte_span_to_lsp_range(text, 9, 13);
    assert_eq!(range.start, Position { line: 1, character: 2 });
    assert_eq!(range.end, Position { line: 1, character: 6 });
}
