# Unity Crop

[Документация на русском](README.ru.md)

Unity Crop is a multithreaded CLI that exports individual sprites from PNG sprite sheets using the slicing data stored in Unity `.meta` files.

## Features

- recursive scanning of a complete Unity project;
- parallel processing of independent sprite sheets;
- each PNG is decoded only once per run;
- repeatable include and exclude glob patterns;
- configurable output directory and filename templates;
- safe collision handling and filename sanitization;
- rectangle validation, useful errors, and an end-of-run summary;
- `--dry-run` preview mode;
- release builds for Windows, Linux, and macOS.

## Quick start

Process one directory:

```console
unity-crop --input ./in --output ./out
```

Process a complete Unity project:

```console
unity-crop \
  --input /path/to/MyGame \
  --output /path/to/ExportedSprites \
  --recursive
```

Unity Crop skips `.git`, `Library`, `Temp`, `Logs`, `obj`, `Build`, and `Builds` by default. It also skips the output directory when it is inside the input tree.

Each source PNG must have an adjacent `<image>.png.meta` file. In Unity, set Sprite Mode to `Multiple` and slice the texture in the Sprite Editor.

## Options

```text
-i, --input <PATH>            PNG or directory; default: in
-o, --output <PATH>           export root; default: out
-r, --recursive               scan subdirectories
-j, --jobs <N>                worker threads; 0 = automatic
    --include <GLOB>           include relative paths; repeatable
    --exclude <GLOB>           exclude relative paths; repeatable
    --no-default-excludes      disable built-in Unity exclusions
    --follow-links             follow directory symlinks
    --path-template <TEMPLATE> directory template inside output
    --name-template <TEMPLATE> PNG filename template
    --collision <MODE>         error | skip | overwrite | rename
    --dry-run                  plan without writing files
    --fail-fast                stop starting work after an error
-v, --verbose                  print every processed path
-h, --help                     full built-in help
-V, --version                  print the version
```

## File masks

`--include` and `--exclude` accept glob patterns matched against paths relative to `--input`:

```console
unity-crop -i ./MyGame -o ./Export -r \
  --include 'Assets/UI/**/*.png' \
  --exclude '**/Drafts/**'
```

Both arguments are repeatable. When `--include` is omitted, `*.png` and `**/*.png` are used.

## Output templates

The following placeholders are available:

| Placeholder | Value |
|---|---|
| `{sprite}` | sprite name from the `.meta` file |
| `{sheet}` | source PNG filename without its extension |
| `{relative_dir}` | PNG directory relative to `--input` |
| `{index}` | one-based sprite index |
| `{x}`, `{y}` | Unity rectangle coordinates |
| `{width}`, `{height}` | sprite dimensions |

For example, preserve the project structure and include the sheet name in every file:

```console
unity-crop -i ./MyGame -o ./Export -r \
  --path-template '{relative_dir}/{sheet}' \
  --name-template '{sheet}_{index}_{sprite}.png'
```

Missing directories are created automatically. Characters invalid on Windows are replaced with `_`. Absolute paths and `..` are rejected, so templates cannot escape the output root.

## Name collisions

- `rename` (default) appends `_2`, `_3`, and so on;
- `skip` preserves the existing file;
- `overwrite` replaces a file that existed before the run, but rejects two current sprites resolving to the same path;
- `error` reports every collision.

Preview a large export before writing anything:

```console
unity-crop -i ./MyGame -o ./Export -r --dry-run --verbose
```

## Parallel processing

A sprite sheet is one unit of parallel work. Each PNG is decoded once, then its sprites are exported sequentially. Independent sheets run concurrently. This speeds up project-wide exports while keeping peak memory bounded.

`--jobs 0` lets the runtime select the worker count. For very large textures, reduce parallelism explicitly, for example `--jobs 4`.

## Build

Install stable Rust, then run:

```console
cargo build --release --locked
cargo test --locked
```

The binary is written to `target/release/unity-crop` (`unity-crop.exe` on Windows).

## License

[MIT](LICENSE)
