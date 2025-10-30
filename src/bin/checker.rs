use std::process::exit;

use log::{error, info};
use pace26checker::{
    bin_forest::BinForest, instance_reader::*, options::*, solution_reader::Solution,
};

fn main() {
    let opts = Opts::process();

    let instance = Instance::read(&opts);

    if opts.solution.is_some() {
        let solution = Solution::read(&opts, instance.num_leaves());

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
