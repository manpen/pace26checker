use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::checks::bin_forest::*;
use crate::io::instance_reader::InstanceReaderError;
use crate::io::{instance_reader::Instance, solution_reader::*};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum CheckerError {
    #[error("Failed to add input tree in line {} to forest: {err}", lineno + 1)]
    TreeInsertion {
        lineno: usize,
        err: TreeInsertionError,
    },

    #[error("Failed to match solution subtree in line {} to instance tree in line {}", sol_lineno+1, inst_lineno + 1)]
    Mismatch {
        inst_lineno: usize,
        sol_lineno: usize,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    InstanceReaderError(#[from] InstanceReaderError),

    #[error(transparent)]
    SolutionReaderError(#[from] SolutionReaderError),
}

pub fn check_instance_only(path: &Path, paranoid: bool) -> Result<Instance, CheckerError> {
    Ok(Instance::read(path, paranoid)?)
}

// Checks feasibility of solution for instance and if successful returns solution size
pub fn check_instance_and_solution(
    instance_path: &Path,
    solution_path: &Path,
    paranoid: bool,
    keep_instance_copy: bool,
) -> Result<(Option<Instance>, Solution, Vec<BinForest>), CheckerError> {
    let mut instance_reader = BufReader::new(File::open(instance_path)?);
    let mut solution_reader = BufReader::new(File::open(solution_path)?);
    check_instance_and_solution_from(
        &mut instance_reader,
        &mut solution_reader,
        paranoid,
        keep_instance_copy,
    )
}

pub fn check_instance_and_solution_from(
    instance_reader: impl BufRead,
    solution_reader: impl BufRead,
    paranoid: bool,
    keep_instance_copy: bool,
) -> Result<(Option<Instance>, Solution, Vec<BinForest>), CheckerError> {
    let instance = Instance::read_from(instance_reader, paranoid)?;
    let instance_clone = keep_instance_copy.then(|| instance.clone());

    let solution = Solution::read_from(solution_reader, instance.num_leaves(), paranoid)?;
    let mut forests = Vec::with_capacity(instance.num_trees() as usize);

    for (lineno, instance_tree) in instance.trees() {
        let mut forest = BinForest::new(instance.num_leaves);

        forest = match forest.add_tree(instance_tree.clone()) {
            Ok(f) => f,
            Err(err) => {
                return Err(CheckerError::TreeInsertion {
                    lineno: *lineno,
                    err,
                });
            }
        };

        for (solution_line, subtree) in solution.trees() {
            if let Some(f) = forest.isolate_tree(subtree) {
                forest = f;
            } else {
                return Err(CheckerError::Mismatch {
                    inst_lineno: *lineno,
                    sol_lineno: *solution_line,
                });
            }
        }

        forests.push(forest);
    }

    debug!("Feasible solution found");

    Ok((instance_clone, solution, forests))
}

// TODO: add unit tests
#[cfg(test)]
mod tests {
    use crate::checks::bin_tree_with_parent::NodeCursor;
    use crate::checks::checker::{check_instance_and_solution, check_instance_only};
    use crate::io::tests::{test_instances, test_instances_directory};
    use pace26io::binary_tree::{TopDownCursor, TreeWithNodeIdx};

    #[test]
    fn check_instance_and_solution_valid() {
        for (input, output) in test_instances("valid") {
            check_instance_and_solution(&input, output.as_ref().unwrap(), false, false).unwrap();
        }
    }

    #[test]
    fn check_instance_and_solution_valid_paranoid() {
        for (input, output) in test_instances("valid") {
            check_instance_and_solution(&input, output.as_ref().unwrap(), true, false).unwrap();
        }
    }

    #[test]
    fn check_instance_and_solution_invalid() {
        for (input, output) in test_instances("invalid") {
            let okay =
                check_instance_and_solution(&input, output.as_ref().unwrap(), false, false).is_ok();
            assert!(!okay);
        }
    }

    #[test]
    fn check_instance_and_solution_invalid_paranoid() {
        for (input, output) in test_instances("invalid") {
            let okay =
                check_instance_and_solution(&input, output.as_ref().unwrap(), true, false).is_ok();
            assert!(!okay);
        }
    }

    #[test]
    fn check_instance_only_invalid_paranoid() {
        for (input, _) in test_instances("instance_only") {
            assert!(check_instance_only(&input, true).is_err(), "{input:?}");
        }
    }

    #[test]
    fn check_instance_only_valid() {
        for (input, _) in test_instances("valid") {
            assert!(check_instance_only(&input, false).is_ok(), "{input:?}");
        }
    }

    #[test]
    fn check_instance_only_valid_paranoid() {
        for (input, _) in test_instances("valid") {
            assert!(check_instance_only(&input, true).is_ok(), "{input:?}");
        }
    }

    #[test]
    fn all_solution_leafs_are_roots() {
        for (input, solution) in test_instances("valid") {
            let (_instance, solution, forests) =
                check_instance_and_solution(&input, solution.unwrap().as_path(), false, true)
                    .unwrap();
            for (i, f) in forests.iter().enumerate() {
                for solution_leaf in solution.trees.iter().filter_map(|(_, t)| t.leaf_label()) {
                    assert!(
                        f.roots()
                            .iter()
                            .filter_map(|t| t.leaf_label())
                            .find(|&l| l == solution_leaf)
                            .is_some(),
                        "Label: {} missing in forest {i}",
                        solution_leaf.0
                    );
                }
            }
        }
    }

    #[test]
    fn roots_tiny01() {
        let dir = test_instances_directory("tiny");

        let (_instance, _solution, forests) = check_instance_and_solution(
            &dir.join("tiny01.in"),
            &dir.join("tiny01.out"),
            false,
            true,
        )
        .unwrap();

        fn collect_node_ids(nodes: &[NodeCursor]) -> Vec<u32> {
            let mut ids: Vec<_> = nodes.iter().map(|c| c.node_idx().0).collect();
            ids.sort();
            return ids;
        }

        assert_eq!(collect_node_ids(forests[0].roots()), vec![4, 6, 7, 8, 11]);
        assert_eq!(collect_node_ids(forests[1].roots()), vec![4, 6, 12, 13, 15]);
    }
}
