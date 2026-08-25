mod cli;
mod discovery;
mod naming;
mod processor;
mod unity_meta_file;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;

fn main() -> ExitCode {
    match run() {
        Ok(has_errors) if has_errors => ExitCode::FAILURE,
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<bool, String> {
    let cli = Cli::parse();
    let discovery = discovery::discover(&cli)?;

    if cli.verbose {
        println!(
            "found {} sprite sheet(s); using {} worker thread(s)",
            discovery.sheets.len(),
            if cli.jobs == 0 {
                rayon::current_num_threads()
            } else {
                cli.jobs
            }
        );
    }

    let summary = processor::process(&cli, discovery)?;
    for error in &summary.errors {
        eprintln!("error: {error}");
    }

    let action = if cli.dry_run { "planned" } else { "written" };
    println!(
        "Done: {} sheet(s) processed, {} sprite(s) {action}, {} skipped, {} PNG(s) without matching .meta, {} error(s), {:.2?}",
        summary.sheets_processed,
        summary.sprites_written,
        summary.sprites_skipped,
        summary.png_without_meta,
        summary.errors.len(),
        summary.elapsed
    );
    if summary.empty_sheets > 0 {
        println!("Note: {} sheet(s) contained no sliced sprites.", summary.empty_sheets);
    }
    if summary.sheets_found == 0 {
        println!("No sliced PNG sprite sheets were found.");
    }
    Ok(!summary.errors.is_empty())
}
