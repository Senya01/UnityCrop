use crate::unity_meta_file::Sprites;
use unity_meta_file::UnityMetaFile;
use std::path::{Path};
mod unity_meta_file;
use std::fs;

fn main() {
    let in_path = Path::new("in");
    ensure_directory_exists(in_path);
    ensure_directory_exists(Path::new("out"));

    let files = get_png_files_in_folder(in_path);
    for file in files.iter() {
        let file_path = in_path.join(file);
        println!("Processing {file}");

        for sprite_data in parse_meta_file(&format!("{}.meta", file_path.display())) {
            crop_image(&file, &file_path, sprite_data);
        }
    }
}

fn crop_image(file : &str, file_path: &Path, sprite_data: Sprites) {
    let mut img = image::open(&file_path).expect("Failed to open image");
    let cropped_img = img.crop(
        sprite_data.rect.x,
        img.height() - sprite_data.rect.y - sprite_data.rect.height,
        sprite_data.rect.width,
        sprite_data.rect.height,
    );

    let out_path = Path::new("out").join(file.split(".").next().unwrap());
    ensure_directory_exists(&out_path);
    let save_path = out_path.join(format!("{}.png", sprite_data.name));
    println!("{}, {}, {}, {}, {}",
             sprite_data.rect.x,
             sprite_data.rect.y,
             sprite_data.rect.width,
             sprite_data.rect.height,
             file_path.display(),
    );

    if let Err(e) = cropped_img.save(&save_path) {
        eprintln!("Failed to save cropped image: {}", e);
        std::process::exit(1);
    }
}

fn parse_meta_file(path: &str) ->  Vec<Sprites> {
    let data = fs::read_to_string(path).expect("File not found");
    let parsed: UnityMetaFile = serde_yaml::from_str(&data).unwrap();

    parsed.texture_importer.sprite_sheet.sprites
}

fn ensure_directory_exists(path: &Path) {
    if !fs::metadata(path).is_ok() {
        fs::create_dir_all(path).unwrap();
    }
}

fn get_png_files_in_folder(path: &Path) -> Vec<String> {
    fs::read_dir(path).expect("Failed to read directory").filter_map(|entry| {
        let entry = entry.expect("Failed to read file");
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.to_lowercase().ends_with(".png") {
            Some(file_name)
        } else {
            None
        }
    }).collect()
}