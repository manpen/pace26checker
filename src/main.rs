use pace26checker::{instance_reader::*, options::*};

fn main() {
    let opts = Opts::process();

    let _instance_trees = read_instance(&opts);
}
