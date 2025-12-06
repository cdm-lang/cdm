package tree_sitter_cdm_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_cdm "github.com/larner-dev/cdm/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_cdm.Language())
	if language == nil {
		t.Errorf("Error loading Contextual Data Models grammar")
	}
}
