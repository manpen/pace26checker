use std::process::exit;

use log::{error, info};
use pace26checker::{
    checks::bin_forest::BinForest, io::instance_reader::*, io::solution_reader::Solution,
};

use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opts {
    #[structopt()]
    pub instance: PathBuf,

    #[structopt()]
    pub solution: Option<PathBuf>,

    #[structopt(short, long)]
    pub quiet: bool,

    #[structopt(short, long)]
    pub paranoid: bool,
}

impl Opts {
    pub fn process() -> Self {
        let opts = Opts::from_args();

        env_logger::builder()
            .filter_level(if opts.quiet {
                log::LevelFilter::Error
            } else {
                log::LevelFilter::Info
            })
            .init();

        opts
    }
}

fn main() {
    let opts = Opts::process();

    let instance = Instance::read(&opts.instance, opts.paranoid);

    if let Some(solution_path) = opts.solution.as_ref() {
        let solution = Solution::read(solution_path, instance.num_leaves(), opts.paranoid);

        for (lineno, instance_tree) in instance.trees() {
            let mut forest = BinForest::new(instance.num_leaves);

            forest = match forest.add_tree(instance_tree.clone()) {
                Ok(f) => f,
                Err(e) => {
                    error!(
                        "Failed to add input tree in line {} to forest: {}",
                        lineno + 1,
                        e
                    );
                    exit(1);
                }
            };

            for subtree in solution.trees() {
                if let Some(f) = forest.isolate_tree(subtree) {
                    forest = f;
                } else {
                    error!(
                        "Failed to match subtrees of solution against input tree in line {}",
                        lineno + 1
                    );
                    exit(1);
                }
            }
        }

        info!("Feasible solution found");
        println!("Trees in solution: {}", solution.trees().len());
    }
}
