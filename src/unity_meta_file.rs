use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UnityMetaFile {
    #[serde(rename = "TextureImporter")]
    pub texture_importer: TextureImporter
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextureImporter {
    #[serde(rename = "spriteSheet")]
    pub sprite_sheet: SpriteSheet
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpriteSheet {
    pub sprites: Vec<Sprites>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sprites {
    pub name: String,
    pub rect: Rect
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32
}