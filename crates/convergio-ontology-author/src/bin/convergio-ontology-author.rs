//! `convergio-ontology-author` — PoC CLI for ADR-0080 ontology
//! authoring. Produces a reviewable ontology draft from documents
//! and/or an intent; never writes to the registry.

use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;
use clap::Parser;
use convergio_ontology_author::{
    author, AuthorOptions, AuthoringRequest, CliProposer, Intent, MarkitdownConverter,
};

/// Generate a standard ontology draft from documents and/or an intent.
#[derive(Parser)]
#[command(name = "convergio-ontology-author", version)]
struct Cli {
    /// Free-form goal describing the ontology to design.
    #[arg(long)]
    prompt: Option<String>,
    /// Target industry / domain (e.g. higher-education).
    #[arg(long)]
    industry: Option<String>,
    /// Concrete use-case (e.g. student-information-system).
    #[arg(long)]
    use_case: Option<String>,
    /// Source document(s) to ground the ontology in (PDF/DOCX/MD/...).
    #[arg(long = "doc")]
    docs: Vec<PathBuf>,
    /// Output directory for the generated artifacts.
    #[arg(long, default_value = "ontology-out")]
    out: PathBuf,
    /// Max proposer attempts (repair loop budget).
    #[arg(long, default_value_t = 3)]
    max_attempts: u32,
    /// Vendor CLI binary used for the LLM step (ADR-0032).
    #[arg(long, default_value = "claude")]
    proposer_bin: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let intent = Intent {
        prompt: cli.prompt.unwrap_or_default(),
        industry: cli.industry.unwrap_or_default(),
        use_case: cli.use_case.unwrap_or_default(),
    };
    let request = AuthoringRequest {
        intent: if intent.is_blank() {
            None
        } else {
            Some(intent)
        },
        documents: cli.docs,
    };

    let converter = MarkitdownConverter::default();
    let proposer = CliProposer {
        bin: cli.proposer_bin,
        args: vec!["-p".to_string()],
    };
    let mut opts = AuthorOptions::new(cli.out);
    opts.max_attempts = cli.max_attempts;
    opts.now = Utc::now();

    match author(&request, &converter, &proposer, &opts) {
        Ok(outcome) => {
            println!(
                "ontology '{}' drafted in {} attempt(s) using {}",
                outcome.draft.name, outcome.attempts, outcome.model_id
            );
            println!("  objects:    {}", outcome.draft.objects.len());
            println!("  properties: {}", outcome.draft.properties.len());
            println!("  links:      {}", outcome.draft.links.len());
            println!("  OWL:        {}", outcome.artifacts.owl.display());
            println!("  provenance: {}", outcome.artifacts.provenance.display());
            println!("  (draft for review — not committed to the registry)");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
