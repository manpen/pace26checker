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
            forest.add_tree(instance_tree.clone());

            for subtree in solution.trees() {
                let is_error = !forest.isolate_tree(subtree);
                if is_error {
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
