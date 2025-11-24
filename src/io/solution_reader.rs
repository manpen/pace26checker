use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use pace26io::{newick::*, pace::reader::*};
use tracing::{debug, error, warn};

use crate::checks::{bin_tree_with_parent::BinTreeWithParentBuilder, lint_leaf_labels_coverage::*};
use thiserror::Error;

pub type Tree = crate::checks::bin_tree_with_parent::NodeCursor;

#[derive(Debug, Error)]
pub enum SolutionReaderError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Error while reading instance: {0}")]
    VisitorError(#[from] SolutionVisitorError),

    #[error("Warning while reading solution (paranoid mode): {0}")]
    VisitorWarning(#[from] SolutionVisitorWarning),
}

pub struct Solution {
    pub trees: Vec<(usize, Tree)>,
    pub stride_lines: Vec<(String, serde_json::Value)>,
}

impl Solution {
    pub fn num_trees(&self) -> usize {
        self.trees.len()
    }

    pub fn trees(&self) -> &[(usize, Tree)] {
        &self.trees
    }

    pub fn read(path: &Path, num_leaves: u32, paranoid: bool) -> Result<Self, SolutionReaderError> {
        debug!("Read solution from {path:?}");
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut visitor = SolutionInputVisitor::process(&mut reader, num_leaves);

        if !visitor.errors.is_empty() || !visitor.warnings.is_empty() {
            for w in &visitor.warnings {
                warn!(" {w}");
            }

            for e in &visitor.errors {
                error!(" {e}");
            }

            if !visitor.errors.is_empty() {
                return Err(SolutionReaderError::VisitorError(visitor.errors.remove(0)));
            }

            if paranoid {
                return Err(SolutionReaderError::VisitorWarning(
                    visitor.warnings.remove(0),
                ));
            }
        }

        Ok(Self {
            trees: std::mem::take(&mut visitor.trees),
            stride_lines: visitor.stride_lines,
        })
    }
}

#[derive(Default)]
pub struct SolutionInputVisitor {
    pub errors: Vec<SolutionVisitorError>,
    pub warnings: Vec<SolutionVisitorWarning>,
    pub trees: Vec<(usize, Tree)>,
    pub stride_lines: Vec<(String, serde_json::Value)>,
}

#[derive(Error, Debug)]
pub enum SolutionVisitorError {
    #[error("Line {} contains invalid Newick string: {newick_error}", lineno + 1)]
    InvalidNewick {
        lineno: usize,
        newick_error: pace26io::newick::ParserError,
    },

    #[error("Solution has invalid leaves: {0}")]
    InvalidLeafLabels(#[from] LeafLintErrors),

    #[error("Line {} has invalid JSON syntax: {0}", lineno + 1)]
    JsonSyntaxError {
        lineno: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error(transparent)]
    PaceParserError(#[from] pace26io::pace::reader::ReaderError),
}

#[derive(Debug, Error, PartialEq)]
pub enum SolutionVisitorWarning {
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
        let mut builder = BinTreeWithParentBuilder::default();
        match builder.parse_newick_from_str(line, Default::default()) {
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

    fn visit_stride_line(&mut self, lineno: usize, _line: &str, key: &str, value: &str) -> Action {
        match serde_json::from_str::<serde_json::Value>(value) {
            Ok(json_value) => {
                self.stride_lines.push((key.to_string(), json_value));
            }
            Err(e) => {
                self.errors
                    .push(SolutionVisitorError::JsonSyntaxError { lineno, source: e });
            }
        }

        Action::Continue
    }
}

impl SolutionInputVisitor {
    pub fn process(reader: &mut impl BufRead, num_leaves: u32) -> SolutionInputVisitor {
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

    assert_raises_error!(
        invalid_stride,
        b"# comment\n#s key: invalid json\n();",
        2,
        SolutionVisitorError::JsonSyntaxError { lineno: 1, .. }
    );
}
