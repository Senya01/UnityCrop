from PIL import Image, ImageDraw
import yaml
import os
import sys

input_path = "in"
output_path = "out"

def ensure_directory_exists(path):
    if not os.path.exists(path):
        os.makedirs(path)

def flip_y_coordinate(y, image_height):
    return image_height - y

def highlight_area(file_path, x, y, width, height):
    image = Image.open(file_path)
    image_height = image.height
    y_flipped = flip_y_coordinate(y, image_height)
    draw = ImageDraw.Draw(image)
    draw.rectangle([x, y_flipped - height, x + width, y_flipped], outline="red", width=1)
    image.show()

def crop_image(file_path, x, y, width, height, save_path):
    print(file_path, x, y, width, height, save_path)
    image = Image.open(file_path)
    image_height = image.height
    y_flipped = flip_y_coordinate(y, image_height)
    cropped_image = image.crop((x, y_flipped - height, x + width, y_flipped))

    directory = os.path.dirname(save_path)
    ensure_directory_exists(directory)
    cropped_image.save(save_path)

def get_png_files_in_folder():
    files = os.listdir(input_path)

    png_files = [file for file in files if file.lower().endswith(".png")]

    return png_files

def main():
    ensure_directory_exists(input_path)
    ensure_directory_exists(output_path)

    for file in get_png_files_in_folder():
        sprite_path = f"{input_path}/{file}"
        with open(f'{sprite_path}.meta', "r") as yaml_file:
            yaml_data = yaml.safe_load(yaml_file)

            save_dir = f"{output_path}/{file.split('.')[0]}"
            if not os.path.exists(save_dir):
                os.makedirs(save_dir)

            for item in yaml_data['TextureImporter']['spriteSheet']['sprites']:
                crop_image(sprite_path, item['rect']['x'], item['rect']['y'], item['rect']['width'], item['rect']['height'], f"{save_dir}/{item['name']}.png")
                # highlight_area(sprite_path, item['rect']['x'], item['rect']['y'], item['rect']['width'], item['rect']['height'])

if __name__ == "__main__":
    main()
    input("Press Enter to exit...")
    sys.exit()
