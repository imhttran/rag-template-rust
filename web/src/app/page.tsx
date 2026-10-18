"use client";

import { useState } from "react";
import type { FormEvent } from "react";

import { AppShell } from "@/components/AppShell";
import { PageTitle } from "@/components/PageTitle";
import { PageHeader } from "@/components/PageHeader";
import { PageFooter } from "@/components/PageFooter";
import { FormStatus, type FormStatusState } from "@/components/FormStatus";
import { MockBanner } from "@/components/MockBanner";
import { SystemStatus } from "@/components/SystemStatus";
import { SCENARIOS, mockIngest, mockQuery } from "@/lib/mock";
import type { IngestResult, QueryResult, Scenario } from "@/lib/mock";

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export default function Page() {
  const [scenario, setScenario] = useState<Scenario>("success");

  const [fileName, setFileName] = useState<string | null>(null);
  const [ingesting, setIngesting] = useState(false);
  const [ingest, setIngest] = useState<IngestResult | null>(null);

  const [question, setQuestion] = useState("");
  const [asking, setAsking] = useState(false);
  const [answer, setAnswer] = useState<QueryResult | null>(null);

  const ingestStatus: FormStatusState = ingesting
    ? { kind: "info", text: "Uploading…" }
    : ingest
      ? ingest.ok
        ? { kind: "ok", text: ingest.message }
        : { kind: "error", text: ingest.message }
      : null;

  const askStatus: FormStatusState = asking
    ? { kind: "info", text: "Retrieving and answering…" }
    : answer && !answer.ok
      ? { kind: "error", text: answer.message }
      : null;

  async function onIngest() {
    if (!fileName || ingesting) return;
    setIngesting(true);
    setIngest(null);
    await delay(scenario === "loading" ? 1500 : 400);
    setIngest(mockIngest(scenario, fileName));
    setIngesting(false);
  }

  async function onAsk(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (asking || question.trim() === "") return;
    setAsking(true);
    setAnswer(null);
    await delay(scenario === "loading" ? 1500 : 400);
    setAnswer(mockQuery(scenario, question.trim()));
    setAsking(false);
  }

  return (
    <AppShell>
      <PageTitle title="RAG Web UI (prototype)" />
      <PageHeader
        title="RAG document Q&amp;A"
        subtitle="Local prototype — Next.js UI only. Ingestion, retrieval, and generation are not wired yet."
      />

      <MockBanner scenario={scenario} />

      <p>
        <label htmlFor="scenario">Demonstration scenario</label>
        <select
          id="scenario"
          value={scenario}
          onChange={(e) => setScenario(e.target.value as Scenario)}
        >
          {SCENARIOS.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </p>

      <SystemStatus api="mock" database="not-checked" model="mock" />

      <section className="card">
        <h2>Document upload</h2>
        <label htmlFor="doc">Choose one supported file (.md, .markdown, .txt, .pdf)</label>
        <input
          id="doc"
          type="file"
          accept=".md,.markdown,.txt,.pdf"
          aria-describedby="doc-meta"
          onChange={(e) => {
            const f = e.target.files && e.target.files[0];
            setFileName(f ? f.name : null);
            setIngest(null);
          }}
        />
        <p id="doc-meta" className="meta">
          {fileName ? `Selected: ${fileName}` : "No file selected."}
        </p>
        <button type="button" onClick={onIngest} disabled={!fileName || ingesting}>
          {ingesting ? "Ingesting…" : "Start ingestion"}
        </button>
        <FormStatus status={ingestStatus} />
        {ingest && ingest.ok && ingest.incompletePages && ingest.incompletePages.length > 0 ? (
          <p className="insufficient">
            Incomplete PDF extraction — pages requiring OCR:{" "}
            <strong>{ingest.incompletePages.join(", ")}</strong>.
          </p>
        ) : null}
      </section>

      <section className="card">
        <h2>Ask a question</h2>
        <form onSubmit={onAsk}>
          <label htmlFor="q">Question</label>
          <input
            id="q"
            type="text"
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            placeholder="What happens if someone misses a payment?"
            aria-describedby="q-hint"
          />
          <p id="q-hint" className="hint">
            Answers are answered only from retrieved context; the prototype simulates it.
          </p>
          <button type="submit" disabled={asking || question.trim() === ""}>
            {asking ? "Asking…" : "Ask"}
          </button>
        </form>
        <FormStatus status={askStatus} />

        <div aria-live="polite">
          {answer && answer.ok && answer.insufficient ? (
            <p className="insufficient">{answer.message}</p>
          ) : null}

          {answer && answer.ok && answer.answer ? (
            <>
              <h3>Answer</h3>
              <p>{answer.answer}</p>
              {answer.sources && answer.sources.length > 0 ? (
                <>
                  <h3>Sources</h3>
                  <ul className="sources">
                    {answer.sources.map((s, i) => (
                      <li key={i}>
                        <span className="cite">
                          {s.source} — {s.section}
                        </span>
                        <span className="page">
                          {s.page === null ? " (no page metadata)" : ` (page ${s.page})`}
                        </span>
                        <p>{s.text}</p>
                      </li>
                    ))}
                  </ul>
                </>
              ) : null}
            </>
          ) : null}
        </div>
      </section>

      <PageFooter meta={<span>Prototype — mock only, no backend</span>} />
    </AppShell>
  );
}
