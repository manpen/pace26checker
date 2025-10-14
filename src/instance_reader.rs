use super::options::Opts;
use core::error;
use std::{fs::File, io::BufReader, process::exit};

use log::{error, info, warn};
use pace26io::{
    newick::{BinTree, BinTreeBuilder, BinaryTreeParser},
    pace::reader::*,
};

#[derive(Default)]
struct InstanceInputVisitor {
    num_errors: usize,
    num_warnings: usize,

    header: Option<(usize, usize)>,
    trees: Vec<BinTree>,
}

impl InstanceVisitor for InstanceInputVisitor {
    fn visit_header(&mut self, num_trees: usize, num_leafs: usize) -> Action {
        assert!(self.header.is_none()); // double headers should be caught by the parser
        self.header = Some((num_trees, num_leafs));

        Action::Continue
    }

    fn visit_tree(&mut self, lineno: usize, line: &str) -> Action {
        if self.header.is_none() {
            error!("Line {} contains tree, but no header read yet.", lineno + 1);
            self.num_errors += 1;
        }

        let mut builder = BinTreeBuilder::new();
        match builder.parse_newick_from_str(line) {
            Ok(tree) => self.trees.push(tree),
            Err(e) => {
                error!(
                    "Error while parsing Newick string in line {}: {e}",
                    lineno + 1
                );
                self.num_errors += 1;
            }
        }

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

pub fn read_instance(opts: &Opts) -> Vec<BinTree> {
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

    if let Some((num_trees, num_leaves)) = visitor.header {
        if num_trees != visitor.trees.len() {
            error!(
                "Header indicated {num_trees} trees, but found {}",
                visitor.trees.len()
            );
            visitor.num_errors += 1;
        }
    } else {
        error!("No header found");
        visitor.num_errors += 1;
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

    visitor.trees
}
