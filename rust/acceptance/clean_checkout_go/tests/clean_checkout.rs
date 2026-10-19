use clean_checkout_go::{
    fenced_bash_blocks, load_setup, run_step, setup_section, stage_clean_checkout,
    REQUIRED_SETUP_COMMANDS,
};

const HEADING: &str = "Clean checkout with Go present";

fn documented_commands() -> Vec<String> {
    let setup = load_setup().unwrap_or_else(|err| panic!("setup guide unavailable: {err}"));
    let body = setup_section(&setup, HEADING)
        .unwrap_or_else(|| panic!("setup guide is missing section: ## {HEADING}"));
    fenced_bash_blocks(body)
        .into_iter()
        .flat_map(|block| {
            block
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn setup_documents_every_required_step() {
    let documented = documented_commands();
    for step in REQUIRED_SETUP_COMMANDS {
        assert!(
            documented.iter().any(|line| line == step),
            "setup section must document the step: {step}"
        );
    }
}

#[test]
fn setup_names_rust_manifest_and_go_present() {
    let setup = load_setup().expect("setup guide");
    let body = setup_section(&setup, HEADING).expect("clean checkout section");
    assert!(
        body.contains("rust/Cargo.toml"),
        "setup section must name rust/Cargo.toml"
    );
    assert!(
        body.to_lowercase().contains("go"),
        "setup section must state that Go stays present"
    );
}

#[test]
fn clean_checkout_runs_documented_setup_with_go_present() {
    if std::process::Command::new("go")
        .arg("version")
        .output()
        .is_err()
    {
        panic!("the Go toolchain must be present for the clean-checkout check");
    }

    let checkout = stage_clean_checkout().expect("stage clean checkout");
    let documented = documented_commands();
    let mut failures = Vec::new();
    for step in REQUIRED_SETUP_COMMANDS {
        assert!(
            documented.iter().any(|line| line == step),
            "step not documented: {step}"
        );
        match run_step(&checkout, step) {
            Ok(status) if status.success() => {}
            Ok(status) => failures.push(format!("{step} exited with {status}")),
            Err(err) => failures.push(err),
        }
    }
    let _ = std::fs::remove_dir_all(&checkout);
    assert!(
        failures.is_empty(),
        "clean-checkout step failures: {failures:#?}"
    );
}
