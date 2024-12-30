use std::{
    fs::{read_to_string, DirEntry},
    path::{Path, PathBuf},
};

use image::Rgb32FImage;
use saigo::STONE_SIZE;
use tch::{Device, Kind, Tensor};

/// The classes of the training data.
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

/// A single set of training data captured from the same board and loaded from a single directory.
pub struct Dataset {
    pub path: PathBuf,
    width: u32,
    height: u32,
    image_names: Vec<String>,
    pub samples: Vec<(Tensor, Tensor)>,
}

impl Dataset {
    /// Loads a dataset from a directory, if the directory contains a training dataset.
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
                        // For each sample, construct a tensor
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

    /// Returns the number of samples in the dataset (before data augmentation).
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns a sample from the dataset.
    /// Data augmentation is applied by using indexes greater than the number of samples.
    pub fn get(&self, index: usize) -> (Tensor, Tensor) {
        let (sample, label) = &self.samples[index % self.len()];
        let transformation = index / self.len();
        let color_permutation = transformation % 6;
        let transformation = transformation / 6;
        let flip = transformation % 2;
        let rotation = transformation / 2;
        let mut sample = PERMUTATIONS.with(|p| sample.index_select(0, &p[color_permutation]));
        if flip == 1 {
            sample = sample.transpose(1, 2);
        }
        sample = sample.rot90(rotation as i64, [1, 2]);
        (sample, label.copy())
    }

    /// Returns a textual description of the location of the sample in the dataset.
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

thread_local! {
    static PERMUTATIONS: [Tensor; 6] = [
        permutation(0, 1, 2),
        permutation(0, 2, 1),
        permutation(1, 0, 2),
        permutation(1, 2, 0),
        permutation(2, 0, 1),
        permutation(2, 1, 0),
    ];
}

/// Creates an indexing tensor used to permute the color channels of a sample.
fn permutation(r: i64, g: i64, b: i64) -> Tensor {
    Tensor::from_slice(&[r, g, b, r + 3, g + 3, b + 3])
}

/// Loads a training image from a label file, returning the name of the image, the image data, and the labels.
fn load_file(
    entry: DirEntry,
    width: u32,
    height: u32,
) -> Option<(String, Rgb32FImage, Vec<Label>)> {
    let path = entry.path();

    // If the file is not a label file, ignore it
    if path.extension()? != "txt" {
        return None;
    }

    // Load the labels
    let labels: Vec<Label> = read_to_string(&path)
        .ok()?
        .chars()
        .map(Label::from)
        .collect();

    // Load the image
    let image = image::open(path.with_extension("png")).ok()?.into_rgb32f();

    // If the image is invalid, ignore it
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
