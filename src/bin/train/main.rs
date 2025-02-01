use clap::Parser;
use dataloader::DataLoader;
use dataset::Dataset;
use saigo::vision_model::VisionModel;
use std::{
    fs::{self, read_dir},
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tch::{
    nn::{self, Module, OptimizerConfig},
    Device, Kind, Tensor,
};

mod dataloader;
mod dataset;

fn main() {
    let args = Args::parse();

    // Load datasets into memory
    println!("Loading datasets...");
    let mut datasets = Vec::new();
    load_datasets_recursively(&args.data, &mut datasets);

    // Show some statistics about the distribution of training data
    if args.stats {
        println!("Calculating statistics...");
        let mut none = 0.0;
        let mut black = 0.0;
        let mut white = 0.0;
        let mut obscured = 0.0;
        let mut total = 0.0;
        for dataset in &datasets {
            for i in 0..dataset.len() {
                total += 1.0;
                let label = dataset.samples[i].1.int64_value(&[]);
                match label {
                    0 => none += 1.0,
                    1 => black += 1.0,
                    2 => white += 1.0,
                    3 => obscured += 1.0,
                    _ => unreachable!(),
                }
            }
        }
        println!("  None: {}", none / total);
        println!("  Black: {}", black / total);
        println!("  White: {}", white / total);
        println!("  Obscured: {}", obscured / total);
        println!("  Total: {}", total);
    }
    println!("Finished loading {} datasets", datasets.len());

    // Handle Ctrl+C to stop training
    let exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let exit_setter = exit.clone();
    ctrlc::set_handler(move || {
        if exit_setter.load(Ordering::Relaxed) {
            process::exit(1);
        }
        println!("\rFinishing epoch...                                ");
        exit_setter.store(true, Ordering::Relaxed);
    })
    .unwrap();
    println!("Press Ctrl+C to exit.");

    // Run the training loop
    let device = Device::Cuda(0);
    let mut vs = nn::VarStore::new(device);
    let model = VisionModel::new(vs.root());
    let mut opt = nn::Sgd::default().build(&vs, 0.001).unwrap();
    opt.set_momentum(0.9);
    opt.set_weight_decay(0.0001);

    let mut status = String::new();
    let mut epoch = 0;
    while !exit.load(Ordering::Relaxed) {
        epoch += 1;
        let mut total_count = 0.0;
        let mut total_loss = 0.0;
        let mut total_acc = 0.0;
        for batch in DataLoader::new(&datasets, 1024, device) {
            let (samples, labels) = batch;
            let outputs = model.forward(&samples);
            let loss = outputs.cross_entropy_for_logits(&labels);
            opt.backward_step(&loss);

            let n = samples.size()[0] as f64;
            total_count += n;
            total_loss += loss.double_value(&[]) * n;
            total_acc += outputs.accuracy_for_logits(&labels).double_value(&[]) * n;
            status = format!(
                "Epoch: {} Loss: {:<.10} Acc: {:<.10}",
                epoch,
                total_loss / total_count,
                total_acc / total_count
            );
            print!("\r{}", status);
        }
        println!();
    }

    vs.freeze();

    // Save the model to a file
    if let Some(mut out) = args.out {
        out.set_extension("safetensors");
        vs.save(&out).unwrap();
        let mut metadata: String = String::new();
        for dataset in &datasets {
            metadata += &format!("{}\n", dataset.path.display());
        }
        metadata += &status;
        fs::write(out.with_extension("txt"), metadata).unwrap();
        println!("Model saved to {}", out.display());
    }

    // Show the samples that were hardest to learn
    if args.inspect {
        println!("Inspecting training data...");
        let mut top10: Vec<(f64, i64, Tensor, String)> = Vec::new();
        for dataset in &datasets {
            for i in 0..dataset.len() {
                let (sample, label) = &dataset.samples[i];
                let output = model.forward(&sample.to(device)).softmax(1, Kind::Float);
                let expected = label.int64_value(&[]);
                let label_acc = output.double_value(&[0, expected]);
                let index = top10
                    .binary_search_by(|(a, _, _, _)| {
                        a.partial_cmp(&label_acc)
                            .unwrap_or(std::cmp::Ordering::Greater)
                    })
                    .unwrap_or_else(|i| i);
                top10.insert(
                    index,
                    (
                        label_acc,
                        expected,
                        output,
                        format!("Dataset: {} {}", dataset.path.display(), dataset.locate(i)),
                    ),
                );
                top10.truncate(10);
            }
        }

        for (_, expected, output, location) in top10 {
            println!("{}", location);
            println!("  Expected: {}", expected);
            println!("  Output: {}", output.to_string(80).unwrap());
        }
    }
}

/// Loads all datasets in the specified directory and its subdirectories.
fn load_datasets_recursively(dir: &Path, datasets: &mut Vec<Dataset>) {
    if let Some(dataset) = Dataset::load(dir) {
        datasets.push(dataset);
    }

    for entry in read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_datasets_recursively(&path, datasets);
        }
    }
}

#[derive(Parser)]
struct Args {
    /// The parent directory containing the training datasets.
    /// Each subdirectory containing a reference.png will be processed as a separate dataset.
    data: PathBuf,

    /// The name of the model file to generate.
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Print training data statistics before training.
    #[arg(short, long)]
    stats: bool,

    /// Display the hardest training samples, in order to search for mislabeled data.
    #[arg(short, long)]
    inspect: bool,
}
