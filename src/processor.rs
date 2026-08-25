use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use image::ImageFormat;
use rayon::prelude::*;

use crate::cli::{Cli, CollisionPolicy};
use crate::discovery::{Discovery, SpriteSheetFiles};
use crate::naming::{render_output_path, validate_templates, NamingContext};
use crate::unity_meta_file::{SpriteData, UnityMetaFile};

#[derive(Debug, Default)]
pub struct Summary {
    pub sheets_found: usize,
    pub sheets_processed: usize,
    pub empty_sheets: usize,
    pub png_without_meta: usize,
    pub sprites_written: usize,
    pub sprites_skipped: usize,
    pub errors: Vec<String>,
    pub elapsed: Duration,
}

#[derive(Debug, Default)]
struct SheetReport {
    processed: bool,
    empty: bool,
    written: usize,
    skipped: usize,
    messages: Vec<String>,
    errors: Vec<String>,
}

pub fn process(cli: &Cli, discovery: Discovery) -> Result<Summary, String> {
    validate_templates(&cli.path_template, &cli.name_template)?;

    if !cli.dry_run {
        fs::create_dir_all(&cli.output).map_err(|error| {
            format!(
                "failed to create output directory {}: {error}",
                cli.output.display()
            )
        })?;
    }

    let started = Instant::now();
    let allocator = OutputAllocator::new(cli.collision);
    let cancelled = AtomicBool::new(false);
    let run = || {
        discovery
            .sheets
            .par_iter()
            .map(|sheet| process_sheet(cli, sheet, &allocator, &cancelled))
            .collect::<Vec<_>>()
    };

    let reports = if cli.jobs == 0 {
        run()
    } else {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.jobs)
            .build()
            .map_err(|error| format!("failed to create a {}-thread pool: {error}", cli.jobs))?
            .install(run)
    };

    let mut summary = Summary {
        sheets_found: discovery.sheets.len(),
        png_without_meta: discovery.png_without_meta,
        errors: discovery.errors,
        elapsed: started.elapsed(),
        ..Summary::default()
    };

    for report in reports {
        summary.sheets_processed += usize::from(report.processed);
        summary.empty_sheets += usize::from(report.empty);
        summary.sprites_written += report.written;
        summary.sprites_skipped += report.skipped;
        if cli.verbose {
            for message in report.messages {
                println!("{message}");
            }
        }
        summary.errors.extend(report.errors);
    }
    summary.elapsed = started.elapsed();
    Ok(summary)
}

fn process_sheet(
    cli: &Cli,
    files: &SpriteSheetFiles,
    allocator: &OutputAllocator,
    cancelled: &AtomicBool,
) -> SheetReport {
    if cli.fail_fast && cancelled.load(Ordering::Relaxed) {
        return SheetReport::default();
    }

    let mut report = SheetReport::default();
    report
        .messages
        .push(format!("sheet: {}", files.image.display()));

    let meta_source = match fs::read_to_string(&files.meta) {
        Ok(source) => source,
        Err(error) => {
            push_error(
                &mut report,
                cancelled,
                format!("{}: failed to read metadata: {error}", files.meta.display()),
            );
            return report;
        }
    };
    let metadata: UnityMetaFile = match serde_yaml::from_str(&meta_source) {
        Ok(metadata) => metadata,
        Err(error) => {
            push_error(
                &mut report,
                cancelled,
                format!("{}: invalid Unity YAML: {error}", files.meta.display()),
            );
            return report;
        }
    };
    let Some(importer) = metadata.texture_importer else {
        report.empty = true;
        report.messages.push(format!(
            "skip: {} has no TextureImporter section",
            files.meta.display()
        ));
        return report;
    };
    if importer.sprite_sheet.sprites.is_empty() {
        report.empty = true;
        report.messages.push(format!(
            "skip: {} has no sliced sprites",
            files.image.display()
        ));
        return report;
    }

    // Decode once per sheet. Version 2 decoded the same image once per sprite.
    let image = match image::open(&files.image) {
        Ok(image) => image,
        Err(error) => {
            push_error(
                &mut report,
                cancelled,
                format!("{}: failed to decode PNG: {error}", files.image.display()),
            );
            return report;
        }
    };
    report.processed = true;

    let sheet_name = files
        .image
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("sheet");

    for (index, sprite) in importer.sprite_sheet.sprites.iter().enumerate() {
        if cli.fail_fast && cancelled.load(Ordering::Relaxed) {
            break;
        }
        match export_sprite(cli, files, sheet_name, index, sprite, &image, allocator) {
            Ok(ExportResult::Written(path)) => {
                report.written += 1;
                report.messages.push(format!("write: {}", path.display()));
            }
            Ok(ExportResult::Skipped(path)) => {
                report.skipped += 1;
                report.messages.push(format!("skip: {}", path.display()));
            }
            Err(error) => push_error(&mut report, cancelled, error),
        }
    }

    report
}

#[allow(clippy::too_many_arguments)]
fn export_sprite(
    cli: &Cli,
    files: &SpriteSheetFiles,
    sheet_name: &str,
    index: usize,
    sprite: &SpriteData,
    image: &image::DynamicImage,
    allocator: &OutputAllocator,
) -> Result<ExportResult, String> {
    validate_rect(sprite, image.width(), image.height(), &files.image)?;
    let bottom = sprite
        .rect
        .y
        .checked_add(sprite.rect.height)
        .ok_or_else(|| {
            format!(
                "{} / {}: invalid Y coordinates",
                files.image.display(),
                sprite.name
            )
        })?;
    let top = image.height().checked_sub(bottom).ok_or_else(|| {
        format!(
            "{} / {}: invalid Y coordinates",
            files.image.display(),
            sprite.name
        )
    })?;

    let context = NamingContext {
        sprite: &sprite.name,
        sheet: sheet_name,
        relative_dir: &files.relative_dir,
        index,
        rect: &sprite.rect,
    };
    let requested = render_output_path(
        &cli.output,
        &cli.path_template,
        &cli.name_template,
        &context,
    )?;
    let Some(output) = allocator.allocate(&requested)? else {
        return Ok(ExportResult::Skipped(requested));
    };

    if cli.dry_run {
        return Ok(ExportResult::Written(output));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("{}: failed to create directory: {error}", parent.display())
        })?;
    }

    let cropped = image.crop_imm(sprite.rect.x, top, sprite.rect.width, sprite.rect.height);
    cropped
        .save_with_format(&output, ImageFormat::Png)
        .map_err(|error| format!("{}: failed to save PNG: {error}", output.display()))?;
    Ok(ExportResult::Written(output))
}

fn validate_rect(sprite: &SpriteData, width: u32, height: u32, image: &Path) -> Result<(), String> {
    if sprite.rect.width == 0 || sprite.rect.height == 0 {
        return Err(format!(
            "{} / {}: sprite rectangle cannot be empty",
            image.display(),
            sprite.name
        ));
    }
    let right = sprite.rect.x.checked_add(sprite.rect.width);
    let top = sprite.rect.y.checked_add(sprite.rect.height);
    if right.is_none_or(|value| value > width) || top.is_none_or(|value| value > height) {
        return Err(format!(
            "{} / {}: rectangle x={} y={} w={} h={} exceeds image {}x{}",
            image.display(),
            sprite.name,
            sprite.rect.x,
            sprite.rect.y,
            sprite.rect.width,
            sprite.rect.height,
            width,
            height
        ));
    }
    Ok(())
}

fn push_error(report: &mut SheetReport, cancelled: &AtomicBool, message: String) {
    cancelled.store(true, Ordering::Relaxed);
    report.errors.push(message);
}

enum ExportResult {
    Written(PathBuf),
    Skipped(PathBuf),
}

struct OutputAllocator {
    collision: CollisionPolicy,
    reserved: Mutex<HashSet<PathBuf>>,
}

impl OutputAllocator {
    fn new(collision: CollisionPolicy) -> Self {
        Self {
            collision,
            reserved: Mutex::new(HashSet::new()),
        }
    }

    fn allocate(&self, requested: &Path) -> Result<Option<PathBuf>, String> {
        let mut reserved = self
            .reserved
            .lock()
            .map_err(|_| "internal output allocator lock was poisoned".to_owned())?;
        let occupied_in_run = reserved.contains(requested);
        let occupied_on_disk = requested.exists();

        match self.collision {
            CollisionPolicy::Error if occupied_in_run || occupied_on_disk => Err(format!(
                "{}: output already exists (use --collision skip, overwrite, or rename)",
                requested.display()
            )),
            CollisionPolicy::Skip if occupied_in_run || occupied_on_disk => Ok(None),
            CollisionPolicy::Overwrite if occupied_in_run => Err(format!(
                "{}: multiple sprites resolve to the same output during this run",
                requested.display()
            )),
            CollisionPolicy::Rename if occupied_in_run || occupied_on_disk => {
                let unique = unique_path(requested, &reserved);
                reserved.insert(unique.clone());
                Ok(Some(unique))
            }
            _ => {
                reserved.insert(requested.to_owned());
                Ok(Some(requested.to_owned()))
            }
        }
    }
}

fn unique_path(requested: &Path, reserved: &HashSet<PathBuf>) -> PathBuf {
    let parent = requested.parent().unwrap_or_else(|| Path::new(""));
    let stem = requested
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("sprite");
    let extension = requested.extension().and_then(|value| value.to_str());

    for suffix in 2.. {
        let filename = match extension {
            Some(extension) => format!("{stem}_{suffix}.{extension}"),
            None => format!("{stem}_{suffix}"),
        };
        let candidate = parent.join(filename);
        if !candidate.exists() && !reserved.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("an unused numeric filename suffix must exist")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::discovery;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn allocator_renames_collisions_in_the_same_run() {
        let allocator = OutputAllocator::new(CollisionPolicy::Rename);
        let path = Path::new("out/sprite.png");
        assert_eq!(allocator.allocate(path).unwrap(), Some(path.to_owned()));
        assert_eq!(
            allocator.allocate(path).unwrap(),
            Some(PathBuf::from("out/sprite_2.png"))
        );
    }

    #[test]
    fn recursively_discovers_and_exports_a_sheet() {
        let root = std::env::temp_dir().join(format!(
            "unity-crop-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let input = root.join("Project");
        let assets = input.join("Assets/UI");
        let output = root.join("Export");
        fs::create_dir_all(&assets).unwrap();

        let image_path = assets.join("icons.v2.png");
        let image = ImageBuffer::from_fn(4, 2, |x, _| {
            if x < 2 {
                Rgba([255_u8, 0, 0, 255])
            } else {
                Rgba([0_u8, 255, 0, 255])
            }
        });
        image.save(&image_path).unwrap();
        fs::write(
            assets.join("icons.v2.png.meta"),
            r#"
TextureImporter:
  spriteSheet:
    sprites:
      - name: red:icon
        rect: {x: 0, y: 0, width: 2, height: 2}
      - name: green_icon
        rect: {x: 2, y: 0, width: 2, height: 2}
"#,
        )
        .unwrap();

        let cli = Cli {
            input,
            output: output.clone(),
            recursive: true,
            jobs: 2,
            include: Vec::new(),
            exclude: Vec::new(),
            no_default_excludes: false,
            follow_links: false,
            path_template: "{relative_dir}/{sheet}".to_owned(),
            name_template: "{index}_{sprite}".to_owned(),
            collision: CollisionPolicy::Rename,
            dry_run: false,
            fail_fast: false,
            verbose: false,
        };

        let found = discovery::discover(&cli).unwrap();
        assert_eq!(found.sheets.len(), 1);
        let summary = process(&cli, found).unwrap();
        assert!(summary.errors.is_empty());
        assert_eq!(summary.sprites_written, 2);
        assert_eq!(
            image::open(output.join("Assets/UI/icons.v2/1_red_icon.png"))
                .unwrap()
                .width(),
            2
        );
        assert!(output.join("Assets/UI/icons.v2/2_green_icon.png").is_file());

        fs::remove_dir_all(root).unwrap();
    }
}
