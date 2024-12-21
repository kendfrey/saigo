use tch::{nn, Tensor};

use crate::STONE_SIZE;

/// ### Network architecture:
///
/// - Six input planes, three for R, G, B of the input image and three for R, G, B of the reference image.
/// - Two convolutional layers producing a single output plane.
/// - Two fully connected layers producing four classification output values for
///   no stone, black stone, white stone, and obscured.
#[derive(Debug)]
pub struct VisionModel {
    conv: nn::Sequential,
    fc: nn::Sequential,
}

const HIDDEN_PLANES: i64 = 8;
const HIDDEN_NODES: i64 = 64;

impl VisionModel {
    pub fn new(p: nn::Path) -> Self {
        Self {
            conv: nn::seq()
                .add(nn::conv2d(&p, 6, HIDDEN_PLANES, 3, Default::default()))
                .add_fn(|xs| xs.relu())
                .add(nn::conv2d(&p, HIDDEN_PLANES, 1, 3, Default::default()))
                .add_fn(|xs| xs.relu()),
            fc: nn::seq()
                .add(nn::linear(
                    &p,
                    ((STONE_SIZE - 4) * (STONE_SIZE - 4)) as i64,
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
        let intermediate = self
            .conv
            .forward(xs)
            .view([-1, ((STONE_SIZE - 4) * (STONE_SIZE - 4)) as i64]);
        self.fc.forward(&intermediate)
    }
}
