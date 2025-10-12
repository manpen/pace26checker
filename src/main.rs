use std::fs::File;
use std::io::BufReader;
use std::{ops, path::Path};

mod bin_tree;

#[derive(StructOpt)]
struct Opts {
    instance: PathBuf,
    solution: Option<PathBuf>,
    quiet: bool,
}

fn main() {
    let opts = Opts::from_args();

    let instace_trees = read_instance(&opts);
}

struct ReaderVisitor {}

impl pace26io::pace::reader::InstanceVisitor for ReaderVisitor {
    fn visit_header(
        &mut self,
        _num_trees: usize,
        _num_leafs: usize,
    ) -> pace26io::pace::reader::Action {
        pace26io::pace::reader::Action::Continue
    }

    fn visit_tree(&mut self, _lineno: usize, _line: &str) -> pace26io::pace::reader::Action {
        pace26io::pace::reader::Action::Continue
    }

    fn visit_line_with_extra_whitespace(
        &mut self,
        _lineno: usize,
        _line: &str,
    ) -> pace26io::pace::reader::Action {
        pace26io::pace::reader::Action::Continue
    }

    fn visit_unrecognized_dash_line(
        &mut self,
        _lineno: usize,
        _line: &str,
    ) -> pace26io::pace::reader::Action {
        pace26io::pace::reader::Action::Continue
    }

    fn visit_unrecognized_line(
        &mut self,
        _lineno: usize,
        _line: &str,
    ) -> pace26io::pace::reader::Action {
        pace26io::pace::reader::Action::Continue
    }
}

fn read_instance(opts: &Opts) -> Vec<bin_tree::BinTree> {
    let file = File::open(&opts.instance).expect("Failed to open instance file");
    let reader = BufReader::new(file);

    // You can now use `reader` to read from the file.
    Vec::new() // placeholder return
}
