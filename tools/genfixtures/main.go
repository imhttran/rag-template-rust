// Command genfixtures generates the RUST-003 section/chunk parity fixtures from
// the pinned Go reference implementation.
//
// It is a read-only consumer of the reference packages: it calls
// document.ParseSections, loader.Dispatch, and chunking.FromSections and records
// their output. It never writes to the reference implementation and never
// changes an expected output to match Rust.
//
// Run from the repository root (the module root):
//
//	go run ./tools/genfixtures
//
// Output: rust/fixtures/parity/section_chunk_parity.json (override with -out).
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"rag-template/internal/chunking"
	"rag-template/internal/document"
	"rag-template/internal/loader"
)

type sectionJSON struct {
	Title   string `json:"title"`
	Content string `json:"content"`
	Page    int    `json:"page"` // 0 == no page metadata (Go sentinel)
}

type chunkJSON struct {
	Section string `json:"section"`
	Content string `json:"content"`
	Index   int    `json:"index"`
	Page    int    `json:"page"`
}

type caseJSON struct {
	Name      string        `json:"name"`
	Kind      string        `json:"kind"` // markdown | text | pdf
	Input     string        `json:"input,omitempty"`
	InputFile string        `json:"input_file,omitempty"`
	Source    string        `json:"source"`
	Language  string        `json:"language"`
	ChunkSize int           `json:"chunk_size"`
	Overlap   int           `json:"overlap"`
	Sections  []sectionJSON `json:"sections"`
	Chunks    []chunkJSON   `json:"chunks"`
	LoadError string        `json:"load_error,omitempty"`
	// IncompletePages are the 1-based pages with no text layer when the loader
	// returned a partial extraction (Go's *IncompleteExtractionError). Empty for
	// a complete extraction and for a hard error.
	IncompletePages []int `json:"incomplete_pages"`
}

type fixtureJSON struct {
	Provenance map[string]string `json:"provenance"`
	Cases      []caseJSON        `json:"cases"`
}

// words builds a space-separated body of n filler words.
func words(prefix string, n int) string {
	parts := make([]string, n)
	for i := range parts {
		parts[i] = fmt.Sprintf("%s%d", prefix, i+1)
	}
	return strings.Join(parts, " ")
}

func gitRev(args ...string) string {
	out, err := exec.Command("git", args...).Output()
	if err != nil {
		return "unknown"
	}
	return strings.TrimSpace(string(out))
}

func main() {
	inputsDir := flag.String("inputs", filepath.Join("rust", "fixtures", "parity", "inputs"), "fixture input directory")
	out := flag.String("out", filepath.Join("rust", "fixtures", "parity", "section_chunk_parity.json"), "output fixture path")
	flag.Parse()

	type spec struct {
		name, kind, inline, file, source, language string
		size, overlap                              int
	}
	specs := []spec{
		{"md-english-sections", "markdown", "## Alpha\none two three four five\n## Beta\nsix seven eight\n## Gamma\nnine ten", "", "sample.md", "english", 10, 3},
		{"md-vietnamese-sections", "markdown", "## Giới thiệu\nXin chào thế giới đây là văn bản kiểm thử\n## Hợp đồng\nĐiều khoản thanh toán và lãi suất", "", "vi.md", "vietnamese", 5, 2},
		{"md-empty", "markdown", "", "", "empty.md", "english", 10, 3},
		{"md-whitespace-only", "markdown", "   \n\t\n  ", "", "ws.md", "english", 10, 3},
		{"md-leading-block", "markdown", "preamble line before any heading\n## Alpha\none two three four", "", "lead.md", "english", 10, 3},
		{"md-boundary-exact-size", "markdown", "## Alpha\n" + words("w", 10), "", "exact.md", "english", 10, 3},
		{"md-boundary-one-over", "markdown", "## Alpha\n" + words("w", 11), "", "over.md", "english", 10, 3},
		{"md-multi-chunk-overlap", "markdown", "## Alpha\n" + words("w", 25), "", "multi.md", "english", 10, 3},
		{"md-repeated-title", "markdown", "## Alpha\n" + words("a", 10) + "\n## Alpha\n" + words("b", 5), "", "repeat.md", "english", 10, 3},
		{"md-heading-edge", "markdown", "## Alpha\n# not a section\n### also not", "", "edge.md", "english", 50, 20},
		{"md-unicode-words", "markdown", "## Tiêu đề\nmột hai ba bốn năm sáu bảy tám chín mười", "", "uni.md", "vietnamese", 4, 1},
		{"md-zero-size", "markdown", "## Alpha\n" + words("w", 5), "", "zero.md", "english", 0, 0},
		{"md-overlap-equals-size", "markdown", "## Alpha\n" + words("w", 12), "", "oeeq.md", "english", 5, 5},
		{"txt-single-section", "text", "", "plain.txt", "plain.txt", "vietnamese", 10, 3},
		{"txt-empty", "text", "", "empty.txt", "empty.txt", "english", 10, 3},
		{"pdf-text-layer", "pdf", "", "text-layer.pdf", "text-layer.pdf", "english", 50, 20},
		{"pdf-mixed", "pdf", "", "mixed-text-scan.pdf", "mixed-text-scan.pdf", "english", 50, 20},
		{"pdf-no-text-layer", "pdf", "", "no-text-layer.pdf", "no-text-layer.pdf", "english", 50, 20},
	}

	var cases []caseJSON
	for _, sp := range specs {
		var (
			sections []document.Section
			loadErr  error
		)
		switch sp.kind {
		case "markdown":
			sections = document.ParseSections(sp.inline)
		default:
			sections, loadErr = loader.Dispatch(filepath.Join(*inputsDir, sp.file))
		}

		c := caseJSON{
			Name:            sp.name,
			Kind:            sp.kind,
			Input:           sp.inline,
			InputFile:       sp.file,
			Source:          sp.source,
			Language:        sp.language,
			ChunkSize:       sp.size,
			Overlap:         sp.overlap,
			Sections:        []sectionJSON{},
			Chunks:          []chunkJSON{},
			IncompletePages: []int{},
		}
		if sp.kind == "markdown" {
			c.InputFile = ""
		} else {
			c.Input = ""
		}
		if loadErr != nil {
			c.LoadError = loadErr.Error()
		}
		var incomplete *loader.IncompleteExtractionError
		if errors.As(loadErr, &incomplete) {
			c.IncompletePages = incomplete.Pages
		}
		for _, s := range sections {
			c.Sections = append(c.Sections, sectionJSON{Title: s.Title, Content: s.Content, Page: s.Page})
		}
		for _, ch := range chunking.FromSections(sections, sp.size, sp.overlap) {
			c.Chunks = append(c.Chunks, chunkJSON{Section: ch.Section, Content: ch.Content, Index: ch.Index, Page: ch.Page})
		}
		cases = append(cases, c)
	}

	fixture := fixtureJSON{
		Provenance: map[string]string{
			"generator":         "tools/genfixtures (Go reference)",
			"module":            "rag-template",
			"source_repository": "https://github.com/imhttran/rag-template-rust",
			"go_version":        runtime.Version(),
			"commit":            gitRev("rev-parse", "HEAD"),
			"commit_short":      gitRev("rev-parse", "--short", "HEAD"),
			"generated_at":      time.Now().UTC().Format(time.RFC3339),
			"note":              "Expected section/chunk output from the pinned Go reference; never edited to match Rust.",
		},
		Cases: cases,
	}

	if err := os.MkdirAll(filepath.Dir(*out), 0o755); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	data, err := json.MarshalIndent(fixture, "", "  ")
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	data = append(data, '\n')
	if err := os.WriteFile(*out, data, 0o644); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	fmt.Printf("wrote %s (%d cases) from go %s @ %s\n", *out, len(cases), runtime.Version(), fixture.Provenance["commit_short"])
}
