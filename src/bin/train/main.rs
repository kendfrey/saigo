use clap::Parser;
use dataloader::DataLoader;
use dataset::Dataset;
use saigo::vision_model::VisionModel;
use std::{
    fs::{self, read_dir},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tch::{
    nn::{self, Module, OptimizerConfig},
    Device,
};

mod dataloader;
mod dataset;

fn main() {
    let args = Args::parse();

    println!("Loading datasets...");
    let mut datasets = Vec::new();
    load_datasets_recursively(&args.data, &mut datasets);
    if args.stats {
        println!("Calculating statistics...");
        let mut none = 0.0;
        let mut black = 0.0;
        let mut white = 0.0;
        let mut obscured = 0.0;
        let mut total = 0.0;
        for dataset in &datasets {
            for i in dataset.indexes() {
                total += 1.0;
                let label = dataset.get(i).1.int64_value(&[]);
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
    }
    println!("Finished loading {} datasets", datasets.len());

    let exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let exit_setter = exit.clone();
    ctrlc::set_handler(move || {
        println!("\rFinishing epoch...                              ");
        exit_setter.store(true, Ordering::Relaxed);
    })
    .unwrap();

    let device = Device::Cuda(0);
    let vs = nn::VarStore::new(device);
    let model = VisionModel::new(vs.root());
    let mut opt = nn::Sgd::default().build(&vs, 0.0001).unwrap();
    opt.set_momentum(0.9);
    opt.set_weight_decay(0.0001);

    let mut status = String::new();
    let mut epoch = 0;
    while !exit.load(Ordering::Relaxed) {
        epoch += 1;
        let mut total_count = 0.0;
        let mut total_loss = 0.0;
        let mut total_acc = 0.0;
        for batch in DataLoader::new(&datasets, 256, device) {
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
}

fn load_datasets_recursively(dir: &PathBuf, datasets: &mut Vec<Dataset>) {
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
}