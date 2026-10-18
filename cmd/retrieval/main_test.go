package main

import (
	"testing"
	"time"
)

// TestParseFiltersRejectsMalformedTimestamps guards the fail-closed contract:
// an unparsable filter must not be silently dropped, because dropping it would
// widen the result set.
func TestParseFiltersRejectsMalformedTimestamps(t *testing.T) {
	tests := []struct {
		name string
		raw  filters
	}{
		{name: "invalid from", raw: filters{IngestedFrom: "not-a-time"}},
		{name: "invalid to", raw: filters{IngestedTo: "2026-13-40T00:00:00Z"}},
		{name: "whitespace from", raw: filters{IngestedFrom: "   "}},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if _, err := parseFilters(test.raw); err == nil {
				t.Fatalf("parseFilters(%#v) = nil error, want a malformed-filter error", test.raw)
			}
		})
	}
}

// TestParseFiltersParsesValidTimestampAndMetadata verifies that valid filters
// are carried through and empty timestamps stay unset.
func TestParseFiltersParsesValidTimestampAndMetadata(t *testing.T) {
	const (
		from = "2026-10-01T00:00:00Z"
		to   = "2026-10-18T12:30:00+07:00"
	)

	got, err := parseFilters(filters{
		Corpus:       " kb ",
		SourcePrefix: "docs/",
		Language:     " de ",
		IngestedFrom: from,
		IngestedTo:   to,
	})
	if err != nil {
		t.Fatalf("parseFilters: %v", err)
	}

	if got.Corpus != "kb" || got.SourcePrefix != "docs/" || got.Language != "de" {
		t.Fatalf("metadata = %#v, want trimmed corpus and preserved source/language", got)
	}

	wantFrom, _ := time.Parse(time.RFC3339, from)
	wantTo, _ := time.Parse(time.RFC3339, to)

	if got.IngestedFrom == nil || !got.IngestedFrom.Equal(wantFrom) {
		t.Fatalf("IngestedFrom = %v, want %v", got.IngestedFrom, wantFrom)
	}

	if got.IngestedTo == nil || !got.IngestedTo.Equal(wantTo) {
		t.Fatalf("IngestedTo = %v, want %v", got.IngestedTo, wantTo)
	}

	empty, err := parseFilters(filters{Corpus: "kb"})
	if err != nil {
		t.Fatalf("parseFilters (empty timestamps): %v", err)
	}

	if empty.IngestedFrom != nil || empty.IngestedTo != nil {
		t.Fatalf("empty timestamps produced filters: %#v", empty)
	}
}
