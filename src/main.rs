use pace26checker::{instance_reader::*, options::*, solution_reader::Solution};

fn main() {
    let opts = Opts::process();

    let instance = Instance::read(&opts);
    let _solution = opts
        .solution
        .as_ref()
        .map(|_| Solution::read(&opts, instance.num_leaves()));
}
