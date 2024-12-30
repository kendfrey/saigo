use rand::{seq::SliceRandom, thread_rng};
use tch::{Device, Tensor};

use crate::dataset::Dataset;

struct SampleIndex {
    dataset: usize,
    index: usize,
}

/// Shuffles and batches training data from multiple datasets.
pub struct DataLoader<'a> {
    device: Device,
    datasets: &'a Vec<Dataset>,
    indexes: Vec<SampleIndex>,
    batch_size: usize,
    index: usize,
}

impl<'a> DataLoader<'a> {
    /// Creates a new data loader.
    pub fn new(datasets: &'a Vec<Dataset>, batch_size: usize, device: Device) -> Self {
        let mut indexes = Vec::new();
        for (i, dataset) in datasets.iter().enumerate() {
            // Data augmentation is done 48-fold
            // (6-fold color permutation and 8-fold geometric transformation)
            indexes.extend((0..dataset.len() * 48).map(|index| SampleIndex { dataset: i, index }));
        }
        indexes.shuffle(&mut thread_rng());
        Self {
            datasets,
            indexes,
            batch_size,
            index: 0,
            device,
        }
    }
}

impl<'a> Iterator for DataLoader<'a> {
    type Item = (Tensor, Tensor);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.indexes.len() {
            return None;
        }

        let end_index = (self.index + self.batch_size).min(self.indexes.len());
        let batch_indexes = &self.indexes[self.index..end_index];
        let (samples, labels): (Vec<_>, Vec<_>) = batch_indexes
            .iter()
            .map(|SampleIndex { dataset, index }| self.datasets[*dataset].get(*index))
            .unzip();
        self.index = end_index;
        Some((
            Tensor::stack(&samples, 0).to(self.device),
            Tensor::stack(&labels, 0).to(self.device),
        ))
    }
}
