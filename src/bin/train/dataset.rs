use std::{
    fs::{read_to_string, DirEntry},
    path::{Path, PathBuf},
};

use image::Rgb32FImage;
use saigo::STONE_SIZE;
use tch::{Device, Kind, Tensor};

#[derive(Clone, Copy)]
pub enum Label {
    None,
    Black,
    White,
    Obscured,
}

impl From<char> for Label {
    fn from(s: char) -> Self {
        match s {
            'B' => Label::Black,
            'W' => Label::White,
            'X' => Label::Obscured,
            _ => Label::None,
        }
    }
}

pub struct Dataset {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub reference: Rgb32FImage,
    pub images: Vec<LabeledImage>,
}

pub struct LabeledImage {
    pub image: Rgb32FImage,
    pub labels: Vec<Label>,
}

#[derive(Clone, Copy)]
pub struct DatasetSampleIndex {
    image: usize,
    x: u32,
    y: u32,
}

impl Dataset {
    pub fn load(dir: &Path) -> Option<Self> {
        let reference = image::open(dir.join("reference.png")).ok()?.into_rgb32f();
        println!("  {}", dir.display());
        let width = reference.width() / STONE_SIZE;
        let height = reference.height() / STONE_SIZE;
        let mut images = Vec::new();

        for entry in dir.read_dir().ok()?.flatten() {
            if let Some(img) = process_file(entry, width, height) {
                images.push(img);
            }
        }

        Some(Dataset {
            path: dir.to_path_buf(),
            width,
            height,
            reference,
            images,
        })
    }

    pub fn indexes(&self) -> Vec<DatasetSampleIndex> {
        let mut indexes = Vec::new();
        for i in 0..self.images.len() {
            for x in 0..self.width {
                for y in 0..self.height {
                    indexes.push(DatasetSampleIndex { image: i, x, y });
                }
            }
        }
        indexes
    }

    pub fn get(&self, index: DatasetSampleIndex) -> (Tensor, Tensor) {
        let image = &self.images[index.image];
        let mut sample = [0.0; (6 * STONE_SIZE * STONE_SIZE) as usize];
        let y0 = index.y * STONE_SIZE;
        let x0 = index.x * STONE_SIZE;
        for y in 0..STONE_SIZE {
            for x in 0..STONE_SIZE {
                let pixel = image.image.get_pixel(x0 + x, y0 + y);
                let ref_pixel = self.reference.get_pixel(x0 + x, y0 + y);
                sample[(y * STONE_SIZE + x) as usize] = pixel[0];
                sample[(STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = pixel[1];
                sample[(2 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = pixel[2];
                sample[(3 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = ref_pixel[0];
                sample[(4 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = ref_pixel[1];
                sample[(5 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = ref_pixel[2];
            }
        }

        let sample = Tensor::from_slice(&sample).view([6, STONE_SIZE as i64, STONE_SIZE as i64]);
        let label = Tensor::scalar_tensor(
            image.labels[(index.y * self.width + index.x) as usize] as i64,
            (Kind::Uint8, Device::Cpu),
        );
        (sample, label)
    }
}

fn process_file(entry: DirEntry, width: u32, height: u32) -> Option<LabeledImage> {
    if entry.path().extension()? != "txt" {
        return None;
    }

    let labels: Vec<Label> = read_to_string(entry.path())
        .ok()?
        .chars()
        .map(Label::from)
        .collect();

    let image = image::open(entry.path().with_extension("png"))
        .ok()?
        .into_rgb32f();

    if image.width() != width * STONE_SIZE
        || image.height() != height * STONE_SIZE
        || labels.len() != (width * height) as usize
    {
        return None;
    }

    Some(LabeledImage { image, labels })
}
