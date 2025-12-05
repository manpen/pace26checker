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
    let instance = Instance::read(instance_path, paranoid)?;
    let instance_clone = keep_instance_copy.then(|| instance.clone());

    let solution = Solution::read(solution_path, instance.num_leaves(), paranoid)?;
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
