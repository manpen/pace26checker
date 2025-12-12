use crate::checks::bin_tree_with_parent::NodeCursor;
use crate::digest::digest_output::{
    DIGEST_BYTES, InstanceDigest, InstanceDigestBuilder, SolutionDigest, SolutionDigestBuilder,
};
use digest::Output;
use pace26io::binary_tree::TopDownCursor;
use pace26io::newick::NewickWriter;
use sha2::{Digest, Sha256};
use std::io::Write;

pub const DIGEST_HEX_DIGITS: usize = 32;
type Algo = Sha256;

/// Computes the digest of an instance. The digest is invariant in the order of trees and
/// swapping of children. The first digit indicates the number approximate number of trees,
/// the second the approximate number of leaves (both in a logarithmic scale). The digest value
/// is returned in hexadecimal representation containing exactly [`DIGEST_HEX_DIGITS`] digits.
///
/// # Warning
/// Modifies the tree by normalizing the order of each inner leaf.
pub fn digest_instance(trees: Vec<NodeCursor>, num_leaves: u32) -> InstanceDigest {
    let num_trees = trees.len() as u32;

    let digest = {
        let mut digests: Vec<_> = trees.into_iter().map(digest_bintree).collect();
        digest_digests(&mut digests)
    };

    // we use a logarithmic scale to indicate the approximate number of trees and leaves
    let tree_score = num_trees.ilog2().saturating_sub(1).min(0xf);
    let leaves_score = num_leaves.ilog(2).saturating_sub(3).min(0xf);

    InstanceDigestBuilder::default()
        .push_u4(tree_score as u8)
        .unwrap()
        .push_u4(leaves_score as u8)
        .unwrap()
        .push_slice(&digest.as_slice()[..DIGEST_BYTES - 1])
        .unwrap()
        .build()
        .unwrap()
}

/// Computes the digest of an instance. The digest is invariant in the order of trees and
/// swapping of children. The first four digits indicate the number of trees in the solution
/// (clamped at 0xffff). The digest value is returned in hexadecimal representation
/// containing exactly [`DIGEST_HEX_DIGITS`] digits.
///
/// Since we assume that this function is only used for feasible solutions, there is no need
/// to include isolated nodes (i.e. a tree where the root is the sole leaf). Such trees are
/// ignored in the input, but can also be omitted. For this reason, the solution's size (including
/// isolated nodes) needs to be passed explicitly.
///
/// # Warning
/// Modifies the tree by normalizing the order of each inner leaf.
pub fn digest_solution(trees: Vec<NodeCursor>, score: u32) -> SolutionDigest {
    let digest = {
        let mut digests: Vec<_> = trees
            .into_iter()
            .filter(|root| !root.is_leaf())
            .map(digest_bintree)
            .collect();
        digest_digests(&mut digests)
    };

    SolutionDigestBuilder::default()
        .push_u16(score.min(0xffff) as u16)
        .unwrap()
        .push_slice(&digest.as_slice()[..DIGEST_BYTES - 2])
        .unwrap()
        .build()
        .unwrap()
}

/// Computes a hash digest for a binary tree.
///
/// # Warning
/// Modifies the tree by normalizing the order of each inner leaf.
fn digest_bintree(tree: NodeCursor) -> Output<Algo> {
    struct WriteAdapter(Algo);
    impl Write for WriteAdapter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.update(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut writer = WriteAdapter(Algo::new());
    tree.normalize_child_order();
    tree.write_newick(&mut writer).unwrap();
    writer.0.finalize()
}

fn digest_digests(digests: &mut [Output<Algo>]) -> Output<Algo> {
    digests.sort_unstable();
    let mut hasher = Algo::new();

    for d in digests {
        hasher.update(d);
    }

    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use crate::checks::bin_tree_with_parent::BinTreeWithParentBuilder;
    use hex_literal::hex;
    use pace26io::binary_tree::NodeIdx;
    use pace26io::newick::BinaryTreeParser;

    #[test]
    fn digest_bintree() {
        const TREE: &str = "((3,4),(2,1));"; // the hash digest below was computed for the string "((1,2),(3,4));"
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str(TREE, NodeIdx::default())
            .unwrap();
        let digest = super::digest_bintree(tree);
        assert_eq!(
            digest[..],
            hex!("5aecb10e41777da0a300dae254d01a2fad3fd892d0b3b553821e2e684194a1f6")
        );
    }

    #[test]
    fn digest_instance() {
        // the following instances need to receive the same digest
        let instances = vec![
            vec!["((3,4),(2,1));", "(1,(2,(3,4)));"],
            vec!["((4,3),(1,2));", "(1,(2,(4,3)));"],
            vec!["(1,(2,(3,4)));", "((3,4),(2,1));"],
            vec!["(1,(2,(4,3)));", "((4,3),(1,2));"],
        ];

        let mut previous_hash = None;

        for instance in instances {
            let trees: Vec<_> = instance
                .iter()
                .map(|&nw| {
                    BinTreeWithParentBuilder::default()
                        .parse_newick_from_str(nw, NodeIdx::default())
                        .unwrap()
                })
                .collect();
            let digests = super::digest_instance(trees, 4);

            if let Some(previous_hash) = previous_hash {
                assert_eq!(digests, previous_hash);
            }

            previous_hash = Some(digests);
        }
    }

    #[test]
    fn digest_solution() {
        // the following instances need to receive the same digest
        let solutions = vec![
            vec!["((3,4),(2,1));", "(5,(6,(7,8)));", "9;"],
            vec!["((3,4),(1,2));", "9;", "(5,(6,(8,7)));"],
            vec!["((3,4),(2,1));", "(5,(6,(8,7)));"],
            vec!["((3,4),(1,2));", "(5,(6,(7,8)));"],
        ];

        let mut previous_hash = None;

        for sol in solutions {
            let trees: Vec<_> = sol
                .iter()
                .map(|&nw| {
                    BinTreeWithParentBuilder::default()
                        .parse_newick_from_str(nw, NodeIdx::default())
                        .unwrap()
                })
                .collect();
            let digests = super::digest_solution(trees, 3);

            if let Some(previous_hash) = previous_hash {
                assert_eq!(digests, previous_hash);
            }

            previous_hash = Some(digests);
        }
    }
}
