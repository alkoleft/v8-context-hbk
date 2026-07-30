use std::env;
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::PathBuf;
use std::process;

use syntax_helper_search::{
    HbkFactSnapshot, HbkFactSnapshotCacheStatus, write_owned_snapshot_lookup_transcript_jsonl,
    write_owned_snapshot_oracle_jsonl,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("snapshot oracle failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or_else(usage_error)?;
    let index_path = PathBuf::from(args.next().ok_or_else(usage_error)?);
    let snapshot = match mode.as_str() {
        "sql-owned" => HbkFactSnapshot::from_path(index_path)?,
        "cache-owned" => {
            let cache_path = PathBuf::from(args.next().ok_or_else(usage_error)?);
            let report = HbkFactSnapshot::from_path_with_binary_cache(index_path, cache_path)?;
            if let HbkFactSnapshotCacheStatus::Rebuilt { reason } = report.status {
                return Err(io::Error::other(format!(
                    "cache-owned oracle requires status Loaded, but cache was rebuilt: {reason}"
                ))
                .into());
            }
            report.snapshot
        }
        _ => return Err(usage_error().into()),
    };
    let content_path = PathBuf::from(args.next().ok_or_else(usage_error)?);
    let lookup_path = PathBuf::from(args.next().ok_or_else(usage_error)?);
    if args.next().is_some() {
        return Err(usage_error().into());
    }

    let content = BufWriter::new(File::create(content_path)?);
    write_owned_snapshot_oracle_jsonl(&snapshot, content)?;
    let lookups = BufWriter::new(File::create(lookup_path)?);
    write_owned_snapshot_lookup_transcript_jsonl(&snapshot, lookups)?;
    Ok(())
}

fn usage_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: dump_hbk_snapshot_oracle \
         sql-owned <index.sqlite> <content.jsonl> <lookups.jsonl> | \
         cache-owned <index.sqlite> <cache.bin> <content.jsonl> <lookups.jsonl>",
    )
}
