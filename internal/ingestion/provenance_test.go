package ingestion

import (
	"strings"
	"testing"
)

// TestProvenanceMatchQueryComparesLanguageAndCorpus guards the columns that
// decide whether an unchanged file may be skipped. Omitting language or corpus
// would let a re-ingest under a new corpus/language silently keep stale rows.
func TestProvenanceMatchQueryComparesLanguageAndCorpus(t *testing.T) {
	query, args := provenanceMatchQuery("docs/a.md", Provenance{
		ContentHash:   "hash-1",
		ChunkerConfig: "size=50,overlap=20",
		EmbedModel:    "fake",
		Language:      "de",
		Corpus:        "corpus-a",
	})

	for _, want := range []string{
		"content_hash = $2",
		"chunker_config = $3",
		"embed_model = $4",
		"COALESCE(language, '') = $5",
		"COALESCE(corpus, '') = $6",
	} {
		if !strings.Contains(query, want) {
			t.Fatalf("query does not contain %q:\n%s", want, query)
		}
	}

	if strings.Contains(query, "embedding_dim = $7") {
		t.Fatalf("unpinned dimension emitted a dimension clause:\n%s", query)
	}

	if len(args) != 6 {
		t.Fatalf("len(args) = %d, want 6", len(args))
	}

	if args[0] != "docs/a.md" || args[4] != "de" || args[5] != "corpus-a" {
		t.Fatalf("args = %#v, want source/language/corpus in positions 0/4/5", args)
	}
}

// TestProvenanceMatchQueryPinsDimension keeps the pinned-dimension branch on $7
// after language and corpus claimed $5 and $6.
func TestProvenanceMatchQueryPinsDimension(t *testing.T) {
	query, args := provenanceMatchQuery("docs/a.md", Provenance{
		ContentHash:   "hash-1",
		ChunkerConfig: "size=50,overlap=20",
		EmbedModel:    "fake",
		Language:      "en",
		Corpus:        "corpus-a",
		Dimension:     768,
	})

	if !strings.Contains(query, "embedding_dim = $7") {
		t.Fatalf("query does not contain the pinned dimension clause:\n%s", query)
	}

	if len(args) != 7 {
		t.Fatalf("len(args) = %d, want 7", len(args))
	}

	if args[6] != 768 {
		t.Fatalf("args[6] = %#v, want 768", args[6])
	}
}
