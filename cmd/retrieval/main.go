// Command retrieval is the retrieval-only producer for the Agentic SOP
// rag-retrieval/v1 contract (RAG-SOP-002). It reads one bounded JSON request
// from stdin and writes one bounded JSON response to stdout. It performs
// retrieval only: it never generates an answer, never calls the answerability
// or reranking packages, and never imports the answer path.
//
// Corpus isolation: the requested corpus MUST be in the trusted allowlist
// (RAG_RETRIEVAL_ALLOWED_CORPORA, comma-separated). An unknown or missing
// corpus is rejected with UNAUTHORIZED_SCOPE, and the corpus filter is applied
// as a SQL pre-filter (before ranking and LIMIT), so rows with a NULL/legacy
// corpus are never returned.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"
	"time"

	"rag-template/internal/config"
	"rag-template/internal/retrieval"
)

const (
	schemaVersion   = "rag-retrieval/v1"
	maxRequestBytes = 1 << 20
	allowedEnv      = "RAG_RETRIEVAL_ALLOWED_CORPORA"
)

type request struct {
	SchemaVersion string  `json:"schema_version"`
	Query         string  `json:"query"`
	TopK          int     `json:"top_k"`
	Filters       filters `json:"filters"`
	DeadlineMS    int     `json:"deadline_ms"`
}

type filters struct {
	Corpus       string `json:"corpus"`
	SourcePrefix string `json:"source_prefix,omitempty"`
	Language     string `json:"language,omitempty"`
	IngestedFrom string `json:"ingested_from,omitempty"`
	IngestedTo   string `json:"ingested_to,omitempty"`
}

type evidence struct {
	ID              string  `json:"id"`
	Corpus          string  `json:"corpus"`
	Source          string  `json:"source"`
	DocumentHash    string  `json:"document_hash"`
	Section         string  `json:"section"`
	Page            int     `json:"page"`
	Text            string  `json:"text"`
	RetrievalMethod string  `json:"retrieval_method"`
	Score           float64 `json:"score"`
	Rank            int     `json:"rank"`
}

type diagnostics struct {
	ElapsedMS int    `json:"elapsed_ms"`
	ErrorCode string `json:"error_code,omitempty"`
}

type response struct {
	SchemaVersion string      `json:"schema_version"`
	Status        string      `json:"status"`
	Evidence      []evidence  `json:"evidence"`
	Truncated     bool        `json:"truncated"`
	Diagnostics   diagnostics `json:"diagnostics"`
}

func main() {
	os.Exit(run())
}

func run() int {
	start := time.Now()
	raw, err := io.ReadAll(io.LimitReader(os.Stdin, maxRequestBytes+1))
	if err != nil {
		emit(response{SchemaVersion: schemaVersion, Status: "MALFORMED", Diagnostics: diagnostics{ErrorCode: "MALFORMED"}})
		return 0
	}
	if len(raw) > maxRequestBytes {
		emit(response{SchemaVersion: schemaVersion, Status: "OVERSIZE", Diagnostics: diagnostics{ErrorCode: "OVERSIZE"}})
		return 0
	}
	var req request
	dec := json.NewDecoder(strings.NewReader(string(raw)))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&req); err != nil {
		emit(response{SchemaVersion: schemaVersion, Status: "MALFORMED", Diagnostics: diagnostics{ErrorCode: "MALFORMED"}})
		return 0
	}
	if req.SchemaVersion != schemaVersion {
		emit(response{SchemaVersion: schemaVersion, Status: "UNSUPPORTED_SCHEMA", Diagnostics: diagnostics{ErrorCode: "UNSUPPORTED_SCHEMA"}})
		return 0
	}
	corpus := strings.TrimSpace(req.Filters.Corpus)
	if corpus == "" || !allowed(corpus) {
		emit(response{SchemaVersion: schemaVersion, Status: "UNAUTHORIZED_SCOPE", Diagnostics: diagnostics{ErrorCode: "UNAUTHORIZED_SCOPE"}})
		return 0
	}

	parsedFilters, err := parseFilters(req.Filters)
	if err != nil {
		emit(response{SchemaVersion: schemaVersion, Status: "MALFORMED", Diagnostics: diagnostics{ErrorCode: "MALFORMED"}})
		return 0
	}

	topK := req.TopK
	if topK <= 0 || topK > 50 {
		topK = 5
	}

	ctx := context.Background()
	timeout := 30 * time.Second
	if req.DeadlineMS > 0 {
		timeout = time.Duration(req.DeadlineMS) * time.Millisecond
	}
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	cfg, err := config.Load()
	if err != nil {
		emitErr(start, "INTERNAL")
		return 0
	}
	embedder, err := cfg.Embedder()
	if err != nil {
		emitErr(start, "UNAVAILABLE")
		return 0
	}
	conn, err := cfg.Connect(ctx)
	if err != nil {
		emitErr(start, "UNAVAILABLE")
		return 0
	}
	defer conn.Close(context.Background())

	vector, err := embedder.Embed(ctx, req.Query)
	if err != nil {
		emitErr(start, "UNAVAILABLE")
		return 0
	}

	opts := retrieval.PipelineOptions{
		CandidateK:    topK * 4,
		FinalK:        topK,
		ExpandLimit:   0,
		MinSimilarity: 0,
		Language:      parsedFilters.Language,
		Filter:        parsedFilters,
	}

	res, err := retrieval.HybridRetrieve(ctx, retrieval.New(conn), req.Query, vector, opts)
	if err != nil {
		if ctx.Err() == context.DeadlineExceeded {
			emitErr(start, "TIMEOUT")
			return 0
		}
		emitErr(start, "INTERNAL")
		return 0
	}

	docs := res.Fused
	if len(docs) == 0 {
		docs = res.Expanded
	}
	out := make([]evidence, 0, len(docs))
	for i, d := range docs {
		if i >= topK {
			break
		}
		score := d.FusionScore
		if score == 0 {
			if d.Similarity != 0 {
				score = d.Similarity
			} else {
				score = d.KeywordScore
			}
		}
		out = append(out, evidence{
			ID:              strconv.FormatInt(d.ID, 10),
			Corpus:          corpus,
			Source:          d.Source,
			DocumentHash:    d.ContentHash,
			Section:         d.Section,
			Page:            d.Page,
			Text:            d.Content,
			RetrievalMethod: "hybrid",
			Score:           score,
			Rank:            i + 1,
		})
	}

	status := "OK"
	if len(out) == 0 {
		status = "EMPTY"
	}
	emit(response{
		SchemaVersion: schemaVersion,
		Status:        status,
		Evidence:      out,
		Truncated:     len(docs) > len(out),
		Diagnostics:   diagnostics{ElapsedMS: int(time.Since(start).Milliseconds())},
	})
	return 0
}

func emitErr(start time.Time, code string) {
	emit(response{SchemaVersion: schemaVersion, Status: code, Diagnostics: diagnostics{ElapsedMS: int(time.Since(start).Milliseconds()), ErrorCode: code}})
}

func emit(r response) {
	enc := json.NewEncoder(os.Stdout)
	if err := enc.Encode(r); err != nil {
		fmt.Fprintf(os.Stderr, "retrieval: encode response: %v\n", err)
	}
}

func allowed(corpus string) bool {
	for _, c := range strings.Split(os.Getenv(allowedEnv), ",") {
		if strings.TrimSpace(c) == corpus {
			return true
		}
	}
	return false
}

// parseFilters maps the request filters onto the retrieval filter. A non-empty
// timestamp that cannot be parsed is an error: silently dropping it would widen
// the result set instead of failing the malformed request.
func parseFilters(raw filters) (retrieval.Filter, error) {
	parsed := retrieval.Filter{
		Corpus:       strings.TrimSpace(raw.Corpus),
		SourcePrefix: raw.SourcePrefix,
		Language:     strings.TrimSpace(raw.Language),
	}

	if raw.IngestedFrom != "" {
		t, err := parseTimestamp(raw.IngestedFrom)
		if err != nil {
			return retrieval.Filter{}, fmt.Errorf("ingested_from: %w", err)
		}

		parsed.IngestedFrom = &t
	}

	if raw.IngestedTo != "" {
		t, err := parseTimestamp(raw.IngestedTo)
		if err != nil {
			return retrieval.Filter{}, fmt.Errorf("ingested_to: %w", err)
		}

		parsed.IngestedTo = &t
	}

	return parsed, nil
}

// parseTimestamp parses an RFC 3339 timestamp, trimming surrounding whitespace.
func parseTimestamp(value string) (time.Time, error) {
	value = strings.TrimSpace(value)
	if value == "" {
		return time.Time{}, fmt.Errorf("empty timestamp")
	}

	return time.Parse(time.RFC3339, value)
}
