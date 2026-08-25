use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum CollisionPolicy {
    /// Stop writing the colliding sprite and report an error.
    Error,
    /// Keep the existing file and skip the new sprite.
    Skip,
    /// Replace a file that existed before this run.
    Overwrite,
    /// Append _2, _3, and so on until the name is unique.
    #[default]
    Rename,
}

#[derive(Debug, Parser)]
#[command(
    name = "unity-crop",
    version,
    about = "Export sliced Unity sprite sheets using their .meta files",
    long_about = None
)]
pub struct Cli {
    /// PNG file or directory to scan.
    #[arg(short, long, default_value = "in")]
    pub input: PathBuf,

    /// Root directory for exported sprites.
    #[arg(short, long, default_value = "out")]
    pub output: PathBuf,

    /// Search all subdirectories under the input directory.
    #[arg(short, long)]
    pub recursive: bool,

    /// Number of worker threads; 0 uses the number of available CPU cores.
    #[arg(short = 'j', long, alias = "threads", default_value_t = 0)]
    pub jobs: usize,

    /// Glob of PNG paths to include, relative to the input directory. Repeatable.
    #[arg(long, value_name = "GLOB", action = ArgAction::Append)]
    pub include: Vec<String>,

    /// Glob of paths to exclude, relative to the input directory. Repeatable.
    #[arg(long, value_name = "GLOB", action = ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Do not apply the built-in Unity project directory exclusions.
    #[arg(long)]
    pub no_default_excludes: bool,

    /// Follow directory symlinks while scanning.
    #[arg(long)]
    pub follow_links: bool,

    /// Output directory template relative to --output.
    #[arg(long, default_value = "{relative_dir}/{sheet}")]
    pub path_template: String,

    /// Output filename template. The .png extension is added when omitted.
    #[arg(long, default_value = "{sprite}.png")]
    pub name_template: String,

    /// How to handle an output path that already exists.
    #[arg(long, value_enum, default_value = "rename")]
    pub collision: CollisionPolicy,

    /// Show planned output paths without creating files or directories.
    #[arg(long)]
    pub dry_run: bool,

    /// Stop starting new work after the first processing error.
    #[arg(long)]
    pub fail_fast: bool,

    /// Print every discovered sheet and exported sprite.
    #[arg(short, long)]
    pub verbose: bool,
}
