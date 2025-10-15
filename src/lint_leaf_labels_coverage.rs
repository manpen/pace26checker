use pace26io::newick::{Label, TopDownCursor};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum LeafLintErrors {
    #[error("Found leaf with label {label}, but expected labels in [1, {expected}]")]
    InvalidLabel { label: u32, expected: u32 },

    #[error("Found more than {expected} leaves")]
    TooManyLeaves { expected: u32 },

    #[error("Found only {found} leaves, but expected {expected}")]
    TooFewLeaves { found: u32, expected: u32 },

    #[error("Found duplicate leaf labels")]
    DuplicateLabels,
}

/// Asserts that all leaf labels in the tree are within the range `[1, expected_num_leaves]`,
/// that there are no duplicate labels, and that there are exactly `expected_num_leaves` leaves.
pub fn assert_leaf_labels_are_within_range<C: TopDownCursor>(
    cursors: impl Iterator<Item = C>,
    expected_num_leaves: u32,
) -> Result<(), LeafLintErrors> {
    fn collect_rec<C: TopDownCursor>(
        cursor: C,
        leaves: &mut Vec<u32>,
        expected_num_leaves: u32,
    ) -> Result<(), LeafLintErrors> {
        if let Some(Label(label)) = cursor.leaf_label() {
            if label < 1 || label > expected_num_leaves {
                return Err(LeafLintErrors::InvalidLabel {
                    label,
                    expected: expected_num_leaves,
                });
            }

            if leaves.len() == expected_num_leaves as usize {
                return Err(LeafLintErrors::TooManyLeaves {
                    expected: expected_num_leaves,
                });
            }

            leaves.push(label);

            Ok(())
        } else if let Some((left, right)) = cursor.children() {
            collect_rec(left, leaves, expected_num_leaves)?;
            collect_rec(right, leaves, expected_num_leaves)
        } else {
            unreachable!("A node is neither a leaf nor has children");
        }
    }

    let mut leaves = Vec::with_capacity(expected_num_leaves as usize);
    for cursor in cursors {
        collect_rec(cursor, &mut leaves, expected_num_leaves)?;
    }

    if leaves.len() < expected_num_leaves as usize {
        return Err(LeafLintErrors::TooFewLeaves {
            found: leaves.len() as u32,
            expected: expected_num_leaves,
        });
    }

    leaves.sort_unstable();
    leaves.dedup();

    if leaves.len() < expected_num_leaves as usize {
        return Err(LeafLintErrors::DuplicateLabels);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bin_tree_with_parent::*;
    use pace26io::newick::*;

    fn lint_tree(s: &str, expected_num_leaves: u32) -> Result<(), LeafLintErrors> {
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str(s)
            .expect("Failed to parse tree");

        assert_leaf_labels_are_within_range(std::iter::once(tree.top_down()), expected_num_leaves)
    }

    #[test]
    fn correct_tree() {
        assert_eq!(lint_tree("((1,2),(3,4));", 4), Ok(()));
    }

    #[test]
    fn too_many() {
        assert_eq!(
            lint_tree("((1,2),(3,3));", 3),
            Err(LeafLintErrors::TooManyLeaves { expected: 3 })
        );
    }

    #[test]
    fn duplicate() {
        assert_eq!(
            lint_tree("((1,2),(3,3));", 4),
            Err(LeafLintErrors::DuplicateLabels)
        );
    }

    #[test]
    fn too_few() {
        assert_eq!(
            lint_tree("((1,2),(3,4));", 5),
            Err(LeafLintErrors::TooFewLeaves {
                found: 4,
                expected: 5
            })
        );
    }

    #[test]
    fn invalid_label() {
        assert_eq!(
            lint_tree("((0,2),(3,4));", 4),
            Err(LeafLintErrors::InvalidLabel {
                label: 0,
                expected: 4
            })
        );
        assert_eq!(
            lint_tree("((1,2),(3,5));", 4),
            Err(LeafLintErrors::InvalidLabel {
                label: 5,
                expected: 4
            })
        );
    }

    #[test]
    fn test_forest() {
        let trees: Vec<NodeCursor> = ["(1,2);", "(3,4);"]
            .iter()
            .map(|s| {
                BinTreeWithParentBuilder::default()
                    .parse_newick_from_str(*s)
                    .expect("Failed to parse tree")
            })
            .collect();

        assert_eq!(
            assert_leaf_labels_are_within_range(trees.iter().map(|t| t.top_down()), 4),
            Ok(())
        );

        assert_eq!(
            assert_leaf_labels_are_within_range(trees.iter().skip(1).map(|t| t.top_down()), 4),
            Err(LeafLintErrors::TooFewLeaves {
                found: 2,
                expected: 4
            })
        );
    }
}
