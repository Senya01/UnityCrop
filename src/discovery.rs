use std::ffi::OsString;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::{DirEntry, WalkDir};

use crate::cli::Cli;

const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    ".git/**",
    "Library",
    "Library/**",
    "Temp",
    "Temp/**",
    "Logs",
    "Logs/**",
    "obj",
    "obj/**",
    "Build",
    "Build/**",
    "Builds",
    "Builds/**",
];

#[derive(Debug)]
pub struct Discovery {
    pub sheets: Vec<SpriteSheetFiles>,
    pub png_without_meta: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpriteSheetFiles {
    pub image: PathBuf,
    pub meta: PathBuf,
    pub relative_dir: PathBuf,
}

pub fn discover(cli: &Cli) -> Result<Discovery, String> {
    if !cli.input.exists() {
        return Err(format!(
            "input path does not exist: {}",
            cli.input.display()
        ));
    }

    let default_includes = ["*.png".to_owned(), "**/*.png".to_owned()];
    let include_patterns = if cli.include.is_empty() {
        &default_includes
    } else {
        cli.include.as_slice()
    };
    let include = build_glob_set(include_patterns, "include")?;

    let mut exclude_patterns = cli.exclude.clone();
    if !cli.no_default_excludes {
        exclude_patterns.extend(DEFAULT_EXCLUDES.iter().map(|value| (*value).to_owned()));
    }
    let exclude = build_glob_set(&exclude_patterns, "exclude")?;

    if cli.input.is_file() {
        let parent = cli.input.parent().unwrap_or_else(|| Path::new("."));
        return Ok(discover_file(&cli.input, parent, &include, &exclude));
    }

    let max_depth = if cli.recursive { usize::MAX } else { 1 };
    let output_absolute = absolute_path(&cli.output)?;
    let input_absolute = absolute_path(&cli.input)?;
    let walker = WalkDir::new(&cli.input)
        .follow_links(cli.follow_links)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| should_visit(entry, &input_absolute, &output_absolute, &exclude));

    let mut discovery = Discovery {
        sheets: Vec::new(),
        png_without_meta: 0,
        errors: Vec::new(),
    };

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                discovery.errors.push(format!("scan error: {error}"));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }

        let relative = match entry.path().strip_prefix(&cli.input) {
            Ok(path) => path,
            Err(_) => entry.path(),
        };
        let relative_match = slash_path(relative);
        if !include.is_match(&relative_match) || exclude.is_match(&relative_match) {
            continue;
        }

        add_file(
            &mut discovery,
            entry.path(),
            relative.parent().unwrap_or_else(|| Path::new("")),
        );
    }

    discovery.sheets.sort_by(|left, right| left.image.cmp(&right.image));
    Ok(discovery)
}

fn discover_file(input: &Path, root: &Path, include: &GlobSet, exclude: &GlobSet) -> Discovery {
    let mut discovery = Discovery {
        sheets: Vec::new(),
        png_without_meta: 0,
        errors: Vec::new(),
    };
    let relative = input.strip_prefix(root).unwrap_or(input);
    let relative_match = slash_path(relative);
    if include.is_match(&relative_match) && !exclude.is_match(&relative_match) {
        add_file(&mut discovery, input, Path::new(""));
    }
    discovery
}

fn add_file(discovery: &mut Discovery, image: &Path, relative_dir: &Path) {
    if image
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("png"))
    {
        return;
    }

    let meta = meta_path_for(image);
    if !meta.is_file() {
        discovery.png_without_meta += 1;
        return;
    }

    discovery.sheets.push(SpriteSheetFiles {
        image: image.to_owned(),
        meta,
        relative_dir: relative_dir.to_owned(),
    });
}

fn should_visit(entry: &DirEntry, input: &Path, output: &Path, exclude: &GlobSet) -> bool {
    let absolute = match absolute_path(entry.path()) {
        Ok(path) => path,
        Err(_) => return true,
    };
    if entry.depth() > 0 && absolute == output {
        return false;
    }
    let relative = absolute.strip_prefix(input).unwrap_or(&absolute);
    !exclude.is_match(slash_path(relative))
}

fn meta_path_for(image: &Path) -> PathBuf {
    let mut value: OsString = image.as_os_str().to_owned();
    value.push(".meta");
    PathBuf::from(value)
}

fn build_glob_set(patterns: &[String], kind: &str) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)
            .map_err(|error| format!("invalid {kind} glob '{pattern}': {error}"))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|error| format!("failed to build {kind} globs: {error}"))
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| format!("failed to resolve {}: {error}", path.display()))
    }
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
