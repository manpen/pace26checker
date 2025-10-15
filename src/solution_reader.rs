use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::exit,
};

use log::{error, info, warn};
use pace26io::{newick::*, pace::reader::*};

use crate::{lint_leaf_labels_coverage::*, options::Opts};
use thiserror::Error;

pub struct Solution {
    pub trees: Vec<BinTree>,
}

impl Solution {
    pub fn num_trees(&self) -> usize {
        self.trees.len()
    }

    pub fn trees(&self) -> &[BinTree] {
        &self.trees
    }

    pub fn read(opts: &Opts, num_leaves: u32) -> Self {
        info!("Read solution from {:?}", opts.solution);
        let file =
            File::open(opts.solution.as_ref().unwrap()).expect("Failed to open Solution file");
        let mut reader = BufReader::new(file);
        let visitor = SolutionInputVisitor::process(&mut reader, num_leaves);

        if !visitor.errors.is_empty() || !visitor.warnings.is_empty() {
            for w in &visitor.warnings {
                warn!(" {w}");
            }

            for e in &visitor.errors {
                error!(" {e}");
            }

            if !visitor.errors.is_empty() || opts.paranoid {
                exit(1);
            }
        }

        Self {
            trees: visitor.trees.into_iter().map(|(_, tree)| tree).collect(),
        }
    }
}

#[derive(Default)]
struct SolutionInputVisitor {
    errors: Vec<SolutionVisitorError>,
    warnings: Vec<SolutionVisitorWarning>,
    trees: Vec<(usize, BinTree)>,
}

#[derive(Error, Debug)]
enum SolutionVisitorError {
    #[error("Line {} contains invalid Newick string: {newick_error}", lineno + 1)]
    InvalidNewick {
        lineno: usize,
        newick_error: pace26io::newick::ParserError,
    },

    #[error("Solution has invalid leaves: {0}")]
    InvalidLeafLabels(#[from] LeafLintErrors),

    #[error(transparent)]
    PaceParserError(#[from] pace26io::pace::reader::ReaderError),
}

#[derive(Debug, Error, PartialEq)]
enum SolutionVisitorWarning {
    #[error("Line {} has extra whitespace", lineno + 1)]
    ExtraWhitespace { lineno: usize },

    #[error("Line {} starts with `#`, but is neither a header ('#p') nor a comment ('# ')", lineno + 1)]
    UnrecognizedDashLine { lineno: usize },

    #[error("Line {} is neither a comment, header, nor a tree", lineno + 1)]
    UnrecognizedLine { lineno: usize },

    #[error("Line {} contains an instance header, but solutions should not provide one", lineno + 1)]
    FoundHeader { lineno: usize },
}

impl InstanceVisitor for SolutionInputVisitor {
    fn visit_header(&mut self, lineno: usize, _num_trees: usize, _num_leafs: usize) -> Action {
        self.warnings
            .push(SolutionVisitorWarning::FoundHeader { lineno });
        Action::Continue
    }

    fn visit_tree(&mut self, lineno: usize, line: &str) -> Action {
        let mut builder = BinTreeBuilder::new();
        match builder.parse_newick_from_str(line) {
            Ok(tree) => self.trees.push((lineno, tree)),
            Err(e) => {
                self.errors.push(SolutionVisitorError::InvalidNewick {
                    lineno,
                    newick_error: e,
                });
            }
        }

        Action::Continue
    }

    fn visit_line_with_extra_whitespace(&mut self, lineno: usize, _line: &str) -> Action {
        self.warnings
            .push(SolutionVisitorWarning::ExtraWhitespace { lineno });
        Action::Continue
    }

    fn visit_unrecognized_dash_line(&mut self, lineno: usize, _line: &str) -> Action {
        self.warnings
            .push(SolutionVisitorWarning::UnrecognizedDashLine { lineno });
        Action::Continue
    }

    fn visit_unrecognized_line(&mut self, lineno: usize, _line: &str) -> Action {
        self.warnings
            .push(SolutionVisitorWarning::UnrecognizedLine { lineno });
        Action::Continue
    }
}

impl SolutionInputVisitor {
    fn process(reader: &mut impl BufRead, num_leaves: u32) -> SolutionInputVisitor {
        let mut visitor = SolutionInputVisitor::default();
        let mut solution_reader = InstanceReader::new(&mut visitor);

        if let Err(e) = solution_reader.read(reader) {
            visitor
                .errors
                .push(SolutionVisitorError::PaceParserError(e));

            return visitor;
        }

        if let Err(e) = assert_leaf_labels_are_within_range(
            visitor.trees.iter().map(|(_, t)| t.top_down()),
            num_leaves,
        ) {
            visitor
                .errors
                .push(SolutionVisitorError::InvalidLeafLabels(e));
        }

        visitor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_raises_error {
        ($name : ident, $str : expr, $num_leaves : expr, $pat : pat) => {
            #[test]
            fn $name() {
                let data = $str;
                let visitor = SolutionInputVisitor::process(&mut &data[..], $num_leaves);
                assert!(visitor.errors.iter().any(|e| matches!(e, $pat)));
            }
        };
    }

    macro_rules! assert_raises_warning {
        ($name : ident, $str : expr, $num_leaves : expr, $pat : pat) => {
            #[test]
            fn $name() {
                let data = $str;
                let visitor = SolutionInputVisitor::process(&mut &data[..], $num_leaves);
                assert!(visitor.warnings.iter().any(|e| matches!(e, $pat)));
            }
        };
    }

    assert_raises_warning!(
        found_header,
        b"# comment\n#p 1 2\n(1,2);\n(1,2);",
        4,
        SolutionVisitorWarning::FoundHeader { lineno: 1 }
    );

    assert_raises_warning!(
        unrecognized_dash_line,
        b"#x 1 1\n(1,2);",
        42,
        SolutionVisitorWarning::UnrecognizedDashLine { lineno: 0 }
    );

    assert_raises_warning!(
        unrecognized_line,
        b"# comment\nrandom text\n(1,2);",
        32,
        SolutionVisitorWarning::UnrecognizedLine { lineno: 1 }
    );

    assert_raises_error!(
        invalid_tree_labels,
        b"(1,3);",
        2,
        SolutionVisitorError::InvalidLeafLabels(..)
    );

    assert_raises_error!(
        invalid_newick,
        b"# comment\n(0,1);\n();",
        4,
        SolutionVisitorError::InvalidNewick { lineno: 2, .. }
    );
}
