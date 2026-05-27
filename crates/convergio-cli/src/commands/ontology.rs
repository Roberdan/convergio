//! `cvg ontology` — read the schema registry maintained by the
//! daemon's Ontology Runtime Core (ADR-0053). Subcommands: `list-types`,
//! `describe object|link <NAME>`, `export <NAME> --format jsonschema|shacl`.
//! Every subcommand respects `--output human|json|plain` per ADR-0043.

use super::{Client, OutputMode};
use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use convergio_i18n::Bundle;
use serde::{Deserialize, Serialize};

/// `cvg ontology` subcommand surface.
#[derive(Subcommand)]
pub enum OntologyCommand {
    /// List the latest revision of every registered ObjectType and LinkType.
    ListTypes,
    /// Describe a single ObjectType or LinkType.
    Describe {
        /// Type family.
        #[arg(value_enum)]
        kind: TypeKindArg,
        /// Registry name.
        name: String,
        /// Pin a specific schema version (default: latest).
        #[arg(long)]
        version: Option<i64>,
    },
    /// Render one ObjectType as a byte-identical export.
    Export {
        /// Object-type registry name.
        name: String,
        /// Export format.
        #[arg(long, value_enum, default_value_t = ExportFormatArg::Jsonschema)]
        format: ExportFormatArg,
        /// Pin a specific schema version (default: latest).
        #[arg(long)]
        version: Option<i64>,
    },
}

/// Type family selector for `cvg ontology describe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum TypeKindArg {
    /// ObjectType — a typed domain object.
    Object,
    /// LinkType — a typed relationship between objects.
    Link,
}

/// Export format selector for `cvg ontology export`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ExportFormatArg {
    /// Canonical JSON-Schema document (draft 2020-12).
    Jsonschema,
    /// SHACL shape graph encoded as JSON-LD.
    Shacl,
}

impl ExportFormatArg {
    fn as_path(self) -> &'static str {
        match self {
            Self::Jsonschema => "jsonschema",
            Self::Shacl => "shacl",
        }
    }
}

#[derive(Deserialize, Serialize)]
struct TypeRow {
    kind: String,
    name: String,
    schema_version: i64,
    title: String,
    description: String,
    content_hash: String,
}

#[derive(Deserialize, Serialize)]
struct ListResponse {
    objects: Vec<TypeRow>,
    links: Vec<TypeRow>,
}

#[derive(Deserialize, Serialize)]
struct PropertyRow {
    name: String,
    schema_version: i64,
    datatype: String,
    required: bool,
    title: String,
    description: String,
    content_hash: String,
}

#[derive(Deserialize, Serialize)]
struct DescribeObject {
    name: String,
    schema_version: i64,
    title: String,
    description: String,
    breaking: bool,
    content_hash: String,
    properties: Vec<PropertyRow>,
}

#[derive(Deserialize, Serialize)]
struct DescribeLink {
    name: String,
    schema_version: i64,
    title: String,
    description: String,
    from_object: String,
    to_object: String,
    breaking: bool,
    content_hash: String,
}

/// Entry point routed from `crate::dispatch`.
pub async fn run(
    client: &Client,
    _bundle: &Bundle,
    output: OutputMode,
    cmd: OntologyCommand,
) -> Result<()> {
    match cmd {
        OntologyCommand::ListTypes => list_types(client, output).await,
        OntologyCommand::Describe {
            kind,
            name,
            version,
        } => match kind {
            TypeKindArg::Object => describe_object(client, output, &name, version).await,
            TypeKindArg::Link => describe_link(client, output, &name, version).await,
        },
        OntologyCommand::Export {
            name,
            format,
            version,
        } => export_object(client, output, &name, format, version).await,
    }
}

async fn list_types(client: &Client, output: OutputMode) -> Result<()> {
    let doc: ListResponse = client.get("/v1/ontology/types").await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&doc)?),
        OutputMode::Plain => {
            for r in &doc.objects {
                println!(
                    "object\t{}\t{}\t{}",
                    r.name, r.schema_version, r.content_hash
                );
            }
            for r in &doc.links {
                println!("link\t{}\t{}\t{}", r.name, r.schema_version, r.content_hash);
            }
        }
        OutputMode::Human => {
            if doc.objects.is_empty() && doc.links.is_empty() {
                println!("(no ontology types registered)");
                return Ok(());
            }
            if !doc.objects.is_empty() {
                println!("ObjectTypes:");
                for r in &doc.objects {
                    println!(
                        "  {:<20} v{}  {}  ({})",
                        r.name,
                        r.schema_version,
                        r.title,
                        &r.content_hash[..12]
                    );
                }
            }
            if !doc.links.is_empty() {
                println!("LinkTypes:");
                for r in &doc.links {
                    println!(
                        "  {:<20} v{}  {}  ({})",
                        r.name,
                        r.schema_version,
                        r.title,
                        &r.content_hash[..12]
                    );
                }
            }
        }
    }
    Ok(())
}

async fn describe_object(
    client: &Client,
    output: OutputMode,
    name: &str,
    version: Option<i64>,
) -> Result<()> {
    let path = match version {
        Some(v) => format!("/v1/ontology/types/object/{name}?version={v}"),
        None => format!("/v1/ontology/types/object/{name}"),
    };
    let doc: DescribeObject = client.get(&path).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&doc)?),
        OutputMode::Plain => {
            println!("{}\t{}\t{}", doc.name, doc.schema_version, doc.content_hash);
            for p in &doc.properties {
                println!(
                    "  {}\t{}\t{}\trequired={}",
                    p.name, p.schema_version, p.datatype, p.required
                );
            }
        }
        OutputMode::Human => {
            println!(
                "object {} v{} {}",
                doc.name,
                doc.schema_version,
                if doc.breaking { "[breaking]" } else { "" }
            );
            println!("  title       : {}", doc.title);
            println!("  description : {}", doc.description);
            println!("  hash        : {}", doc.content_hash);
            if doc.properties.is_empty() {
                println!("  properties  : (none)");
            } else {
                println!("  properties  :");
                for p in &doc.properties {
                    println!(
                        "    - {} ({}{}) v{}",
                        p.name,
                        p.datatype,
                        if p.required { ", required" } else { "" },
                        p.schema_version
                    );
                }
            }
        }
    }
    Ok(())
}

async fn describe_link(
    client: &Client,
    output: OutputMode,
    name: &str,
    version: Option<i64>,
) -> Result<()> {
    let path = match version {
        Some(v) => format!("/v1/ontology/types/link/{name}?version={v}"),
        None => format!("/v1/ontology/types/link/{name}"),
    };
    let doc: DescribeLink = client.get(&path).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&doc)?),
        OutputMode::Plain => println!(
            "{}\t{}\t{}\t{}->{}",
            doc.name, doc.schema_version, doc.content_hash, doc.from_object, doc.to_object
        ),
        OutputMode::Human => {
            println!("link {} v{}", doc.name, doc.schema_version);
            println!("  title       : {}", doc.title);
            println!("  description : {}", doc.description);
            println!("  endpoints   : {} -> {}", doc.from_object, doc.to_object);
            println!("  hash        : {}", doc.content_hash);
        }
    }
    Ok(())
}

async fn export_object(
    client: &Client,
    output: OutputMode,
    name: &str,
    format: ExportFormatArg,
    version: Option<i64>,
) -> Result<()> {
    let path = match version {
        Some(v) => format!(
            "/v1/ontology/export/{}/object/{}?version={}",
            format.as_path(),
            name,
            v
        ),
        None => format!("/v1/ontology/export/{}/object/{}", format.as_path(), name),
    };
    let bytes = client.get_bytes(&path).await?;
    // The export bytes are already canonical JSON with a trailing
    // newline — print verbatim to preserve byte-identity. Human and
    // Plain modes match Json here so piping to a file always works.
    let _ = output;
    use std::io::Write;
    std::io::stdout().write_all(&bytes)?;
    Ok(())
}
