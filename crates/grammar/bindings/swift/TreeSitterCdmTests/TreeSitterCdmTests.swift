import XCTest
import SwiftTreeSitter
import TreeSitterCdm

final class TreeSitterCdmTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_cdm())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Contextual Data Models grammar")
    }
}
