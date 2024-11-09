use anyhow::Result;
use clap::Args;
use clap_stdin::MaybeStdin;
use log::info;
use std::path::PathBuf;
use timetracksync::{parse_records, upload};

#[derive(Debug, Args, Clone)]
pub struct IngestArgs {
    file: PathBuf,

    embedded_username: String,
    embedded_password: MaybeStdin<String>,
}

pub(crate) async fn handle(args: IngestArgs) -> Result<()> {
    info!("Ingesting {:?}", args.file);
    let records = parse_records(&args.file).expect("Failed to parse records");

    info!("Ingested {} records", records.len());

    upload(
        records,
        &args.embedded_username,
        &args.embedded_password.to_string(),
    )
    .await?;

    Ok(())
}
