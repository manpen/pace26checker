use crate::lint_leaf_labels_coverage::*;

use super::options::Opts;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::exit,
};

use log::{error, info, warn};
use pace26io::{newick::*, pace::reader::*};
use thiserror::Error;

pub struct Instance {
    pub trees: Vec<BinTree>,
    pub num_leaves: u32,
}

impl Instance {
    pub fn num_trees(&self) -> u32 {
        self.trees.len() as u32
    }

    pub fn num_leaves(&self) -> u32 {
        self.num_leaves
    }

    pub fn trees(&self) -> &[BinTree] {
        &self.trees
    }

    pub fn read(opts: &Opts) -> Self {
        info!("Read instance from {:?}", opts.instance);
        let file = File::open(&opts.instance).expect("Failed to open instance file");
        let mut reader = BufReader::new(file);
        let visitor = InstanceInputVisitor::process(&mut reader);

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
            num_leaves: visitor.header.unwrap().1,
            trees: visitor.trees.into_iter().map(|(_, tree)| tree).collect(),
        }
    }
}

//////////////////////////////////////////////////////////////////

#[derive(Default)]
struct InstanceInputVisitor {
    errors: Vec<InstanceVisitorError>,
    warnings: Vec<InstanceVisitorWarning>,

    header: Option<(u32, u32)>,
    trees: Vec<(usize, BinTree)>,
}

#[derive(Error, Debug)]
enum InstanceVisitorError {
    #[error("Line {} contains tree, but no header read yet.", lineno + 1)]
    NoHeaderBeforeFirstTree { lineno: usize },

    #[error("No header found in the input")]
    NoHeaderFound,

    #[error("Line {} contains invalid Newick string: {newick_error}", lineno + 1)]
    InvalidNewick {
        lineno: usize,
        newick_error: pace26io::newick::ParserError,
    },

    #[error("Header indicates {expected} trees, but found {found}")]
    TreeCountMismatch { expected: usize, found: usize },

    #[error("Tree {tree_index} in line {} has invalid leaf labels: {lint_error}", lineno + 1)]
    InvalidLeafLabels {
        lineno: usize,
        tree_index: usize,
        lint_error: LeafLintErrors,
    },

    #[error("Line {} starts with `#`, but is neither a header ('#p') nor a comment ('# ')", lineno + 1)]
    UnrecognizedDashLine { lineno: usize },

    #[error("Line {} is neither a comment, header, nor a tree", lineno + 1)]
    UnrecognizedLine { lineno: usize },

    #[error(transparent)]
    PaceParserError(#[from] pace26io::pace::reader::ReaderError),
}

#[derive(Debug, Error, PartialEq)]
enum InstanceVisitorWarning {
    #[error("Line {} has extra whitespace", lineno + 1)]
    ExtraWhitespace { lineno: usize },
}

impl InstanceVisitor for InstanceInputVisitor {
    fn visit_header(&mut self, _lineno: usize, num_trees: usize, num_leafs: usize) -> Action {
        assert!(self.header.is_none()); // double headers should be caught by the parser
        self.header = Some((num_trees as u32, num_leafs as u32));

        Action::Continue
    }

    fn visit_tree(&mut self, lineno: usize, line: &str) -> Action {
        if self.header.is_none() {
            self.errors
                .push(InstanceVisitorError::NoHeaderBeforeFirstTree { lineno });
        }

        let mut builder = BinTreeBuilder::new();
        match builder.parse_newick_from_str(line) {
            Ok(tree) => self.trees.push((lineno, tree)),
            Err(e) => {
                self.errors.push(InstanceVisitorError::InvalidNewick {
                    lineno,
                    newick_error: e,
                });
            }
        }

        Action::Continue
    }

    fn visit_line_with_extra_whitespace(&mut self, lineno: usize, _line: &str) -> Action {
        self.warnings
            .push(InstanceVisitorWarning::ExtraWhitespace { lineno });
        Action::Continue
    }

    fn visit_unrecognized_dash_line(&mut self, lineno: usize, _line: &str) -> Action {
        self.errors
            .push(InstanceVisitorError::UnrecognizedDashLine { lineno });
        Action::Continue
    }

    fn visit_unrecognized_line(&mut self, lineno: usize, _line: &str) -> Action {
        self.errors
            .push(InstanceVisitorError::UnrecognizedLine { lineno });
        Action::Continue
    }
}

impl InstanceInputVisitor {
    fn process(reader: &mut impl BufRead) -> InstanceInputVisitor {
        let mut visitor = InstanceInputVisitor::default();
        let mut instance_reader = InstanceReader::new(&mut visitor);

        if let Err(e) = instance_reader.read(reader) {
            visitor
                .errors
                .push(InstanceVisitorError::PaceParserError(e));

            return visitor;
        }

        if let Some((num_trees, num_leaves)) = visitor.header {
            if num_trees as usize != visitor.trees.len() {
                visitor
                    .errors
                    .push(InstanceVisitorError::TreeCountMismatch {
                        expected: num_trees as usize,
                        found: visitor.trees.len(),
                    });
            }

            for (i, (lineno, tree)) in visitor.trees.iter().enumerate() {
                if let Err(e) = assert_leaf_labels_are_within_range(
                    std::iter::once(tree.top_down()),
                    num_leaves,
                ) {
                    visitor
                        .errors
                        .push(InstanceVisitorError::InvalidLeafLabels {
                            tree_index: i + 1,
                            lineno: *lineno,
                            lint_error: e,
                        });
                }
            }
        } else {
            visitor.errors.push(InstanceVisitorError::NoHeaderFound);
        }

        visitor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_reader_no_header() {
        let data = b"((1,2),(3,4));";
        let visitor = InstanceInputVisitor::process(&mut &data[..]);

        assert_eq!(visitor.errors.len(), 2, "Errors: {:#?}", visitor.errors);
        assert!(matches!(
            visitor.errors[0],
            InstanceVisitorError::NoHeaderBeforeFirstTree { lineno: 0 }
        ));
        assert!(matches!(
            visitor.errors[1],
            InstanceVisitorError::NoHeaderFound
        ));
    }

    macro_rules! assert_raises_error {
        ($name : ident, $str : expr, $pat : pat) => {
            #[test]
            fn $name() {
                let data = $str;
                let visitor = InstanceInputVisitor::process(&mut &data[..]);
                assert!(visitor.errors.iter().any(|e| matches!(e, $pat)));
            }
        };
    }

    assert_raises_error!(
        missing_tree,
        b"#p 2 2\n(1,2);",
        InstanceVisitorError::TreeCountMismatch {
            expected: 2,
            found: 1
        }
    );

    assert_raises_error!(
        missing_tree_too_many,
        b"#p 2 2\n(1,2);\n(1,2);\n(1,2);",
        InstanceVisitorError::TreeCountMismatch {
            expected: 2,
            found: 3
        }
    );

    assert_raises_error!(
        no_header_before_first,
        b"# comment\n(1,2);\n#p 2 2\n(1,2);",
        InstanceVisitorError::NoHeaderBeforeFirstTree { lineno: 1 }
    );

    assert_raises_error!(
        no_header,
        b"# comment\n(1,2);\n(1,2);",
        InstanceVisitorError::NoHeaderFound
    );

    assert_raises_error!(
        unrecognized_dash_line,
        b"#x 1 1\n(1,2);",
        InstanceVisitorError::UnrecognizedDashLine { lineno: 0 }
    );

    assert_raises_error!(
        unrecognized_line,
        b"# comment\nrandom text\n(1,2);",
        InstanceVisitorError::UnrecognizedLine { lineno: 1 }
    );

    assert_raises_error!(
        invalid_tree_labels,
        b"#p 1 2\n(1,3);",
        InstanceVisitorError::InvalidLeafLabels {
            tree_index: 1,
            lineno: 1,
            lint_error: LeafLintErrors::InvalidLabel {
                label: 3,
                expected: 2
            }
        }
    );

    assert_raises_error!(
        reader_error,
        b"#p 1 2\n#p 1 2\n(1,2);",
        InstanceVisitorError::PaceParserError(..)
    );

    assert_raises_error!(
        invalid_newick,
        b"#p 1 1\n(0,1);\n();",
        InstanceVisitorError::InvalidNewick { lineno: 2, .. }
    );
}
