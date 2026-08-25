use std::path::{Component, Path, PathBuf};

use crate::unity_meta_file::SpriteRect;

const TOKENS: &[&str] = &[
    "{sprite}",
    "{sheet}",
    "{relative_dir}",
    "{index}",
    "{x}",
    "{y}",
    "{width}",
    "{height}",
];

pub struct NamingContext<'a> {
    pub sprite: &'a str,
    pub sheet: &'a str,
    pub relative_dir: &'a Path,
    pub index: usize,
    pub rect: &'a SpriteRect,
}
pub fn validate_templates(path_template: &str, name_template: &str) -> Result<(), String> {
    validate_tokens(path_template)?;
    validate_tokens(name_template)?;

    if Path::new(path_template).is_absolute() {
        return Err("--path-template must be relative to --output".to_owned());
    }
    if path_template
        .replace('\\', "/")
        .split('/')
        .any(|component| component == "..")
    {
        return Err("--path-template cannot contain '..'".to_owned());
    }
    if name_template.contains('/') || name_template.contains('\\') {
        return Err("--name-template cannot contain path separators".to_owned());
    }
    Ok(())
}

pub fn render_output_path(
    output_root: &Path,
    path_template: &str,
    name_template: &str,
    context: &NamingContext<'_>,
) -> Result<PathBuf, String> {
    let rendered_dir = render(path_template, context, true);
    let mut rendered_name = render(name_template, context, false);
    if !rendered_name.to_ascii_lowercase().ends_with(".png") {
        rendered_name.push_str(".png");
    }
    if rendered_name == ".png" {
        return Err("the filename template produced an empty name".to_owned());
    }

    let relative_dir = safe_relative_path(&rendered_dir)?;
    let result = output_root.join(relative_dir).join(rendered_name);
    Ok(result)
}

fn render(template: &str, context: &NamingContext<'_>, allow_directory: bool) -> String {
    let relative_dir = context
        .relative_dir
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(sanitize_component(&value.to_string_lossy())),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    let mut value = template.to_owned();
    let replacements = [
        ("{sprite}", sanitize_component(context.sprite)),
        ("{sheet}", sanitize_component(context.sheet)),
        ("{relative_dir}", relative_dir),
        ("{index}", (context.index + 1).to_string()),
        ("{x}", context.rect.x.to_string()),
        ("{y}", context.rect.y.to_string()),
        ("{width}", context.rect.width.to_string()),
        ("{height}", context.rect.height.to_string()),
    ];
    for (token, replacement) in replacements {
        value = value.replace(token, &replacement);
    }

    if allow_directory {
        value
    } else {
        sanitize_component(&value)
    }
}

fn validate_tokens(template: &str) -> Result<(), String> {
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after_start = &rest[start..];
        let Some(end) = after_start.find('}') else {
            return Err(format!("unclosed template token in '{template}'"));
        };
        let token = &after_start[..=end];
        if !TOKENS.contains(&token) {
            return Err(format!("unknown template token '{token}'"));
        }
        rest = &after_start[end + 1..];
    }
    if rest.contains('}') {
        return Err(format!("unmatched '}}' in template '{template}'"));
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> Result<PathBuf, String> {
    let normalized = value.replace('\\', "/");
    let mut result = PathBuf::new();
    for component in normalized.split('/') {
        match component {
            "" | "." => {}
            ".." => return Err("rendered path attempts to leave the output directory".to_owned()),
            value => result.push(sanitize_component(value)),
        }
    }
    Ok(result)
}

fn sanitize_component(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        {
            result.push('_');
        } else {
            result.push(character);
        }
    }
    let trimmed = result.trim().trim_end_matches(['.', ' ']);
    let mut result = if trimmed.is_empty() {
        "unnamed".to_owned()
    } else {
        trimmed.to_owned()
    };

    let stem = result
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if reserved {
        result.insert(0, '_');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> SpriteRect {
        SpriteRect {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        }
    }

    #[test]
    fn renders_and_sanitizes_output_path() {
        let rect = rect();
        let context = NamingContext {
            sprite: "hero:idle",
            sheet: "characters.v2",
            relative_dir: Path::new("Assets/UI"),
            index: 2,
            rect: &rect,
        };
        let path = render_output_path(
            Path::new("out"),
            "{relative_dir}/{sheet}",
            "{index}_{sprite}",
            &context,
        )
        .unwrap();
        assert_eq!(
            path,
            Path::new("out/Assets/UI/characters.v2/3_hero_idle.png")
        );
    }

    #[test]
    fn rejects_unknown_tokens_and_parent_paths() {
        assert!(validate_templates("{unknown}", "{sprite}").is_err());
        assert!(validate_templates("../{sheet}", "{sprite}").is_err());
    }

    #[test]
    fn protects_windows_reserved_names() {
        assert_eq!(sanitize_component("CON"), "_CON");
        assert_eq!(sanitize_component("hello?.png"), "hello_.png");
    }
}
