// Interop helper for the RUST-005C cross-language database test.
// Modes: write <hash> | read <hash> | delete <hash>. Uses DATABASE_URL.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"strings"

	"github.com/jackc/pgx/v5"
)

func vecLiteral(n int) string {
	parts := make([]string, n)
	for i := range parts {
		parts[i] = "0"
	}
	parts[0], parts[n-1] = "0.25", "0.75"
	return "[" + strings.Join(parts, ",") + "]"
}

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "usage: interop write|read|delete <hash>")
		os.Exit(2)
	}
	mode, hash := os.Args[1], os.Args[2]
	ctx := context.Background()
	conn, err := pgx.Connect(ctx, os.Getenv("DATABASE_URL"))
	if err != nil {
		fmt.Fprintln(os.Stderr, "connect:", err)
		os.Exit(1)
	}
	defer conn.Close(ctx)

	switch mode {
	case "write":
		_, err = conn.Exec(ctx,
			`INSERT INTO documents (content, source, section, chunk_index, language, page, corpus,
			 content_hash, embed_model, embedding_dim, chunker_config, ingested_at, embedding)
			 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12::text::timestamptz,$13::text::vector)`,
			"interop row written by Go", "interop://go/"+hash, "Interop", 0, "english", 1,
			"interop", hash, "go-test-model", 768, "{}", "2026-10-17T00:00:00Z", vecLiteral(768))
	case "read":
		var content, section, lang, model, emb string
		var page, dim int
		err = conn.QueryRow(ctx,
			`SELECT content, COALESCE(section,''), COALESCE(language,''), embed_model,
			 COALESCE(page,0), embedding_dim, embedding::text
			 FROM documents WHERE content_hash=$1 LIMIT 1`, hash).
			Scan(&content, &section, &lang, &model, &page, &dim, &emb)
		if err != nil {
			fmt.Fprintln(os.Stderr, "read:", err)
			os.Exit(1)
		}
		n := 0
		if strings.HasPrefix(emb, "[") && strings.HasSuffix(emb, "]") {
			n = len(strings.Split(strings.Trim(emb, "[]"), ","))
		}
		b, _ := json.Marshal(map[string]any{
			"content": content, "section": section, "language": lang, "embed_model": model,
			"page": page, "embedding_dim": dim, "embedding_len": n,
		})
		fmt.Println(string(b))
	case "delete":
		_, err = conn.Exec(ctx, `DELETE FROM documents WHERE content_hash=$1`, hash)
	default:
		fmt.Fprintln(os.Stderr, "unknown mode", mode)
		os.Exit(2)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, mode+":", err)
		os.Exit(1)
	}
	fmt.Println("ok")
	_ = strconv.Itoa
}
