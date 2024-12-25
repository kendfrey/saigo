use std::{
    fs::{read_to_string, DirEntry},
    ops::Index,
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
    width: u32,
    height: u32,
    image_names: Vec<String>,
    samples: Vec<(Tensor, Tensor)>,
}

impl Dataset {
    pub fn load(dir: &Path) -> Option<Self> {
        let reference = image::open(dir.join("reference.png")).ok()?.into_rgb32f();
        println!("  {}", dir.display());
        let width = reference.width() / STONE_SIZE;
        let height = reference.height() / STONE_SIZE;
        let mut image_names = Vec::new();
        let mut samples = Vec::new();

        for entry in dir.read_dir().ok()?.flatten() {
            if let Some((name, img, labels)) = load_file(entry, width, height) {
                for iy in 0..height {
                    for ix in 0..width {
                        let mut sample = [0.0; (6 * STONE_SIZE * STONE_SIZE) as usize];
                        let x0 = ix * STONE_SIZE;
                        let y0 = iy * STONE_SIZE;
                        for y in 0..STONE_SIZE {
                            for x in 0..STONE_SIZE {
                                let pixel = img.get_pixel(x0 + x, y0 + y);
                                let ref_pixel = reference.get_pixel(x0 + x, y0 + y);
                                sample[(y * STONE_SIZE + x) as usize] = pixel[0];
                                sample[(STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] =
                                    pixel[1];
                                sample
                                    [(2 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] =
                                    pixel[2];
                                sample
                                    [(3 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] =
                                    ref_pixel[0];
                                sample
                                    [(4 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] =
                                    ref_pixel[1];
                                sample
                                    [(5 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] =
                                    ref_pixel[2];
                            }
                        }
                        let sample = Tensor::from_slice(&sample).view([
                            6,
                            STONE_SIZE as i64,
                            STONE_SIZE as i64,
                        ]);
                        let label = Tensor::scalar_tensor(
                            labels[(iy * width + ix) as usize] as i64,
                            (Kind::Uint8, Device::Cpu),
                        );
                        samples.push((sample, label));
                    }
                }
                image_names.push(name);
            }
        }

        Some(Dataset {
            path: dir.to_path_buf(),
            width,
            height,
            image_names,
            samples,
        })
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn locate(&self, index: usize) -> String {
        let index = index as u32;
        let x = index % self.width;
        let index = index / self.width;
        let y = index % self.height;
        let image = index / self.height;
        format!(
            "Image: {} X: {} Y: {}",
            self.image_names[image as usize], x, y
        )
    }
}

impl Index<usize> for Dataset {
    type Output = (Tensor, Tensor);
    fn index(&self, index: usize) -> &Self::Output {
        &self.samples[index]
    }
}

fn load_file(
    entry: DirEntry,
    width: u32,
    height: u32,
) -> Option<(String, Rgb32FImage, Vec<Label>)> {
    let path = entry.path();
    if path.extension()? != "txt" {
        return None;
    }

    let labels: Vec<Label> = read_to_string(&path)
        .ok()?
        .chars()
        .map(Label::from)
        .collect();

    let image = image::open(path.with_extension("png")).ok()?.into_rgb32f();

    if image.width() != width * STONE_SIZE
        || image.height() != height * STONE_SIZE
        || labels.len() != (width * height) as usize
    {
        return None;
    }

    Some((
        path.file_stem().unwrap().to_string_lossy().to_string(),
        image,
        labels,
    ))
}
