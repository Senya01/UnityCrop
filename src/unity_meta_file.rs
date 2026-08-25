use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UnityMetaFile {
    #[serde(rename = "TextureImporter")]
    pub texture_importer: Option<TextureImporter>,
}

#[derive(Debug, Deserialize)]
pub struct TextureImporter {
    #[serde(rename = "spriteSheet", default)]
    pub sprite_sheet: SpriteSheet,
}

#[derive(Debug, Default, Deserialize)]
pub struct SpriteSheet {
    #[serde(default)]
    pub sprites: Vec<SpriteData>,
}

#[derive(Debug, Deserialize)]
pub struct SpriteData {
    pub name: String,
    pub rect: SpriteRect,
}

#[derive(Debug, Deserialize)]
pub struct SpriteRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_unity_texture_meta_fragment() {
        let yaml = r#"
fileFormatVersion: 2
guid: abc123
TextureImporter:
  spriteSheet:
    sprites:
      - name: hero_idle
        rect:
          x: 4
          y: 8
          width: 16
          height: 32
"#;
        let metadata: UnityMetaFile = serde_yaml::from_str(yaml).unwrap();
        let sprites = metadata.texture_importer.unwrap().sprite_sheet.sprites;
        assert_eq!(sprites.len(), 1);
        assert_eq!(sprites[0].name, "hero_idle");
        assert_eq!(sprites[0].rect.height, 32);
    }
}
