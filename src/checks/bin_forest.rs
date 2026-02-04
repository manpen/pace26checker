use super::bin_tree_with_parent::*;
use pace26io::binary_tree::*;
use thiserror::Error;

pub struct BinForest {
    roots: Vec<NodeCursor>,
    leaves: Vec<WeakNodeCursor>,
}

#[derive(Error, Debug)]
pub enum TreeInsertionError {
    #[error("Root is already present in the forest")]
    RootAlreadyPresent,

    #[error("Leaf {leaf_label} is not in required range of [1, {num_leaves}]")]
    LeafOutOfRange { leaf_label: u32, num_leaves: u32 },

    #[error("Leaf {leaf_label} is already present in the forest")]
    LeafAlreadyPresent { leaf_label: u32 },
}

impl BinForest {
    pub fn new(num_leaves: u32) -> Self {
        Self {
            roots: Vec::new(),
            leaves: vec![WeakNodeCursor::default(); 1 + num_leaves as usize],
        }
    }

    /// Adds a tree to the forest. Returns error if the tree is incompatible,
    /// or already (partially) present in the forest. In this case, the forest
    /// is consumed.
    ///
    /// # Remark
    /// If an error occurs, the forest is left in an undefined state and should
    /// not be used further. Hence, we take ownership of self and only return it
    /// on success.
    pub fn add_tree(mut self, root_in: NodeCursor) -> Result<Self, TreeInsertionError> {
        if self.roots.iter().any(|r| r == &root_in) {
            return Err(TreeInsertionError::RootAlreadyPresent);
        }

        for node in root_in.top_down().dfs() {
            if let Some(Label(label)) = node.leaf_label() {
                let label = label as usize;

                if label == 0 || label as u32 > (self.leaves.len() - 1) as u32 {
                    return Err(TreeInsertionError::LeafOutOfRange {
                        leaf_label: label as u32,
                        num_leaves: (self.leaves.len() - 1) as u32,
                    });
                }

                if self.leaves[label].upgrade().is_some() {
                    return Err(TreeInsertionError::LeafAlreadyPresent {
                        leaf_label: label as u32,
                    });
                }

                self.leaves[label] = node.downgrade();
            }
        }

        self.roots.push(root_in);

        Ok(self)
    }

    /// Attempts to extract a subtree according to the MAF rules.
    /// Any non-matched siblings of the subtree become their own root.
    /// Returns the updated forest if successful, otherwise the forest
    /// is consumed.
    ///
    /// # Remark
    /// If an error occurs, the forest is left in an undefined state and should
    /// not be used further. Hence, we take ownership of self and only return it
    /// on success.
    pub fn isolate_tree(mut self, other: &NodeCursor) -> Option<Self> {
        if let Some(root) = self.isolate_tree_match(other) {
            root.update_topology_subtree();
            self.add_root(root);
            Some(self)
        } else {
            None
        }
    }

    fn add_root(&mut self, root: NodeCursor) {
        if self.roots.contains(&root) {
            return;
        }
        let mut builder = BinTreeWithParentBuilder::default();
        let root = builder.make_root(root);
        self.roots.push(root);
    }

    fn isolate_tree_match(&mut self, other: &NodeCursor) -> Option<NodeCursor> {
        if let Some((left, right)) = other.children() {
            let match_left = self.isolate_tree_match(&left)?;
            let match_right = self.isolate_tree_match(&right)?;
            let lca = NodeCursor::lowest_common_ancestor(match_left.clone(), match_right.clone())?;

            if lca.depth() < other.depth() {
                return None;
            }

            self.contract_path(&match_left, &lca);
            self.contract_path(&match_right, &lca);

            Some(lca)
        } else if let Some(Label(l)) = other.leaf_label() {
            self.leaves[l as usize].upgrade()
        } else {
            unreachable!()
        }
    }

    fn contract_path(&mut self, lower: &NodeCursor, upper: &NodeCursor) {
        debug_assert!(lower.depth() > upper.depth());
        for _ in (upper.depth() + 1)..lower.depth() {
            let sibling = lower.remove_sibling().unwrap();
            self.add_root(sibling);
        }
    }

    /// Slice of all roots in the forest
    pub fn roots(&self) -> &[NodeCursor] {
        &self.roots
    }

    pub fn leaf(&self, label: Label) -> WeakNodeCursor {
        self.leaves[label.0 as usize].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pace26io::newick::{BinaryTreeParser, NewickWriter};

    #[test]
    fn add_tree() {
        let tree1 = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,3),(5,7));", Default::default())
            .unwrap();
        let tree2 = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("(2,(4,(6,8)));", Default::default())
            .unwrap();

        let mut forest = BinForest::new(8);
        forest = forest.add_tree(tree1).unwrap();
        forest = forest.add_tree(tree2).unwrap();

        assert!(forest.leaves[0].upgrade().is_none());
        for (i, depth) in [2, 1, 2, 2, 2, 3, 2, 3].iter().enumerate() {
            let i = i + 1;
            assert_eq!(
                forest.leaves[i].upgrade().unwrap().leaf_label(),
                Some(Label(i as u32))
            );

            assert_eq!(forest.leaves[i].upgrade().unwrap().depth(), *depth);
        }
    }

    #[test]
    fn isolate_tree_success() {
        let host = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("(((1,2),(3,4)),(5,(6,7)));", Default::default())
            .unwrap();

        let pattern = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("(((1,2),3),5);", Default::default())
            .unwrap();

        let mut forest = BinForest::new(7);
        forest = forest.add_tree(host).unwrap();
        forest = forest.isolate_tree(&pattern).unwrap();

        // sort roots by the smallest leafs in them
        forest.roots.sort_by_key(|c| {
            c.top_down()
                .dfs()
                .filter_map(|u| u.leaf_label().map(|l| l.0))
                .min()
                .unwrap()
        });

        assert_eq!(
            forest.roots[0].top_down().to_newick_string(),
            "(((1,2),3),5);"
        );
        assert_eq!(forest.roots[1].top_down().to_newick_string(), "4;");
        assert_eq!(forest.roots[2].top_down().to_newick_string(), "(6,7);");
    }

    #[test]
    fn isolate_tree_failed() {
        let host = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("(((1,2),(3,4)),(5,(6,7)));", Default::default())
            .unwrap();

        let pattern = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,5),3);", Default::default())
            .unwrap();

        let mut forest = BinForest::new(7);
        forest = forest.add_tree(host).unwrap();
        assert!(forest.isolate_tree(&pattern).is_none());
    }
}
