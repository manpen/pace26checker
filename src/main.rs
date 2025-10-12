use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::exit;

use log::{error, info, warn};
use pace26io::newick::BinTree;
use structopt::StructOpt;

use pace26io::pace::reader::*;

#[derive(StructOpt)]
struct Opts {
    #[structopt()]
    instance: PathBuf,

    #[structopt()]
    solution: Option<PathBuf>,

    #[structopt(short, long)]
    quiet: bool,

    #[structopt(short, long)]
    paranoid: bool,
}

fn main() {
    let opts = Opts::from_args();

    env_logger::builder()
        .filter_level(if opts.quiet {
            log::LevelFilter::Error
        } else {
            log::LevelFilter::Info
        })
        .init();

    let _instance_trees = read_instance(&opts);
}

#[derive(Default)]
struct InstanceInputVisitor {
    num_errors: usize,
    num_warnings: usize,
}

impl InstanceVisitor for InstanceInputVisitor {
    fn visit_header(&mut self, _num_trees: usize, _num_leafs: usize) -> Action {
        Action::Continue
    }

    fn visit_tree(&mut self, _lineno: usize, _line: &str) -> Action {
        Action::Continue
    }

    fn visit_line_with_extra_whitespace(&mut self, lineno: usize, _line: &str) -> Action {
        warn!(" Line {} has extra whitespace", lineno + 1);
        self.num_warnings += 1;
        Action::Continue
    }

    fn visit_unrecognized_dash_line(&mut self, lineno: usize, _line: &str) -> Action {
        error!(
            " Line {} starts with `#`, but is neither a header ('#p') nor a comment ('# ')",
            lineno + 1
        );
        self.num_errors += 1;
        Action::Continue
    }

    fn visit_unrecognized_line(&mut self, lineno: usize, _line: &str) -> Action {
        error!(" Line {} has unknown type", lineno + 1);
        self.num_errors += 1;
        Action::Continue
    }
}

fn read_instance(opts: &Opts) -> Vec<BinTree> {
    info!("Read instance from {:?}", opts.instance);
    let file = File::open(&opts.instance).expect("Failed to open instance file");
    let reader = BufReader::new(file);

    let mut visitor = InstanceInputVisitor::default();
    let mut instance_reader = InstanceReader::new(&mut visitor);
    let parser_result = instance_reader.read(reader);

    if let Err(e) = parser_result {
        error!("Failed to read instance: {e}");
        exit(1);
    }

    if visitor.num_errors > 0 || visitor.num_warnings > 0 {
        info!(
            "Found {} errors and {} warnings",
            visitor.num_errors, visitor.num_warnings
        );
        if visitor.num_errors > 0 || opts.paranoid {
            exit(1);
        }
    }

    // You can now use `reader` to read from the file.
    Vec::new() // placeholder return
}
