use image::Rgb32FImage;
use tch::{nn, Tensor};

use crate::STONE_SIZE;

/// ### Network architecture:
///
/// - Six input planes, three for R, G, B of the input image and three for R, G, B of the reference image.
/// - Two convolutional layers producing an output with two planes.
/// - Two fully connected layers producing four classification output values for
///   no stone, black stone, white stone, and obscured.
#[derive(Debug)]
pub struct VisionModel {
    conv: nn::Sequential,
    fc: nn::Sequential,
}

const HIDDEN_PLANES: i64 = 8;
const INTERMEDIATE_PLANES: i64 = 2;
const HIDDEN_NODES: i64 = 64;

impl VisionModel {
    pub fn new(p: nn::Path) -> Self {
        Self {
            conv: nn::seq()
                .add(nn::conv2d(&p, 6, HIDDEN_PLANES, 3, Default::default()))
                .add_fn(|xs| xs.relu())
                .add(nn::conv2d(
                    &p,
                    HIDDEN_PLANES,
                    INTERMEDIATE_PLANES,
                    3,
                    Default::default(),
                ))
                .add_fn(|xs| xs.relu()),
            fc: nn::seq()
                .add(nn::linear(
                    &p,
                    INTERMEDIATE_PLANES * ((STONE_SIZE - 4) * (STONE_SIZE - 4)) as i64,
                    HIDDEN_NODES,
                    Default::default(),
                ))
                .add_fn(|xs| xs.relu())
                .add(nn::linear(&p, HIDDEN_NODES, 4, Default::default())),
        }
    }
}

impl nn::Module for VisionModel {
    fn forward(&self, xs: &Tensor) -> Tensor {
        let intermediate = self.conv.forward(xs).view([
            -1,
            INTERMEDIATE_PLANES * ((STONE_SIZE - 4) * (STONE_SIZE - 4)) as i64,
        ]);
        self.fc.forward(&intermediate)
    }
}

pub const LBL_NONE: u8 = 0;
pub const LBL_BLACK: u8 = 1;
pub const LBL_WHITE: u8 = 2;
pub const LBL_OBSCURED: u8 = 3;

/// Constructs an input tensor from the given location in the image.
pub fn read_tensor(image: &Rgb32FImage, reference: &Rgb32FImage, x0: u32, y0: u32) -> Tensor {
    let mut data = [0.0; (6 * STONE_SIZE * STONE_SIZE) as usize];
    for y in 0..STONE_SIZE {
        for x in 0..STONE_SIZE {
            let pixel = image.get_pixel(x0 + x, y0 + y);
            let ref_pixel = reference.get_pixel(x0 + x, y0 + y);
            data[(y * STONE_SIZE + x) as usize] = pixel[0];
            data[(STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = pixel[1];
            data[(2 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = pixel[2];
            data[(3 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = ref_pixel[0];
            data[(4 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = ref_pixel[1];
            data[(5 * STONE_SIZE * STONE_SIZE + y * STONE_SIZE + x) as usize] = ref_pixel[2];
        }
    }
    Tensor::from_slice(&data).view([6, STONE_SIZE as i64, STONE_SIZE as i64])
}
