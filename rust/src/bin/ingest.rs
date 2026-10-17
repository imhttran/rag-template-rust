//! Thin `ingest` entry point. Delegates to the library configuration layer.

fn main() {
    rag_foundation::config::Config::load().unwrap_or_else(|err| {
        eprintln!("config: {err}");
        std::process::exit(1);
    });
}
