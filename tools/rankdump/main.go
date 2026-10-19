// Command rankdump prints the deterministic fused retrieval ranking for every
// evaluation case at the configured FINAL_K.
//
// It exists only for Go/Rust parity evidence: `cmd/eval` prints per-case metrics
// but not the answerable ranking, and RUST-007E must compare source/section
// order and tie ordering against the pinned Go reference. This command is
// evaluation-only and changes no production path. It reuses the same
// configuration, embedder, SQL corpus, and `retrieval.HybridRetrieve` pipeline
// as `cmd/eval`.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"

	"rag-template/internal/config"
	"rag-template/internal/retrieval"
)

type ExpectedDocument struct {
	Source   string   `json:"source"`
	Section  string   `json:"section"`
	Evidence []string `json:"evidence,omitempty"`
}

type EvalCase struct {
	Question      string             `json:"question"`
	Expected      []ExpectedDocument `json:"expected"`
	ExpectedFacts []string           `json:"expected_facts,omitempty"`
}

const defaultDatasetPath = "evals/retrieval.json"

func datasetPath() string {
	if path := os.Getenv("EVAL_DATASET"); path != "" {
		return path
	}

	return defaultDatasetPath
}

func main() {
	if err := run(context.Background()); err != nil {
		log.Fatal(err)
	}
}

func run(ctx context.Context) error {
	data, err := os.ReadFile(datasetPath())
	if err != nil {
		return fmt.Errorf("read dataset: %w", err)
	}

	var cases []EvalCase
	if err := json.Unmarshal(data, &cases); err != nil {
		return fmt.Errorf("parse dataset: %w", err)
	}

	cfg, err := config.Load()
	if err != nil {
		return err
	}

	conn, err := cfg.Connect(ctx)
	if err != nil {
		return err
	}
	defer conn.Close(context.Background())

	embedder, err := cfg.Embedder()
	if err != nil {
		return err
	}

	retriever := retrieval.New(conn)
	options := retrieval.PipelineOptions{
		CandidateK:    cfg.TopK,
		FinalK:        cfg.FinalK,
		ExpandLimit:   cfg.ExpandLimit,
		MinSimilarity: cfg.MinSimilarity,
		Language:      cfg.Language,
	}

	for index, evalCase := range cases {
		vector, err := embedder.Embed(ctx, evalCase.Question)
		if err != nil {
			return fmt.Errorf("embed case %d: %w", index, err)
		}

		result, err := retrieval.HybridRetrieve(
			ctx,
			retriever,
			evalCase.Question,
			vector,
			options,
		)
		if err != nil {
			return fmt.Errorf("retrieve case %d: %w", index, err)
		}

		fmt.Printf("CASE %d\t%s\n", index, evalCase.Question)
		if len(result.Fused) == 0 {
			fmt.Println("UNANSWERABLE")
			continue
		}

		for rank, doc := range result.Fused {
			fmt.Printf(
				"RANK %d\t%s\t%s\t%d\t%.6f\n",
				rank+1,
				doc.Source,
				doc.Section,
				doc.ChunkIndex,
				doc.FusionScore,
			)
		}
	}

	return nil
}
