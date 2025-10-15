use crate::bin_tree_with_parent::*;
use pace26io::binary_tree::*;

pub struct BinForest {
    roots: Vec<NodeCursor>,
    leaves: Vec<WeakNodeCursor>,
}

impl BinForest {
    pub fn new(num_leaves: u32) -> Self {
        Self {
            roots: Vec::new(),
            leaves: vec![WeakNodeCursor::default(); 1 + num_leaves as usize],
        }
    }

    pub fn add_tree(&mut self, root_in: NodeCursor) {
        assert!(!self.roots.iter().any(|r| r == &root_in));

        for node in root_in.top_down().dfs() {
            if let Some(Label(label)) = node.leaf_label() {
                let label = label as usize;
                assert!(label > 0);
                assert!(self.leaves.len() > label);
                assert!(self.leaves[label].upgrade().is_none()); // otherwise we already have this leaf in the forest
                self.leaves[label] = node.downgrade();
            }
        }

        self.roots.push(root_in);
    }

    /// Attempts to extract a subtree according to the MAF rules.
    /// Any non-matches sibling of the subtree becomes it's own root.
    pub fn isolate_tree(&mut self, other: &NodeCursor) -> bool {
        if let Some(root) = self.isolate_tree_match(other) {
            root.update_topology_subtree();
            true
        } else {
            false
        }
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
        let mut builder = BinTreeWithParentBuilder::default();
        for _ in (upper.depth() + 1)..lower.depth() {
            let sibling = lower.remove_sibling().unwrap();
            self.roots.push(builder.make_root(sibling));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pace26io::newick::{BinaryTreeParser, NewickWriter};

    #[test]
    fn add_tree() {
        let tree1 = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,3),(5,7));")
            .unwrap();
        let tree2 = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("(2,(4,(6,8)));")
            .unwrap();

        let mut forest = BinForest::new(8);
        forest.add_tree(tree1);
        forest.add_tree(tree2);

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
            .parse_newick_from_str("(((1,2),(3,4)),(5,(6,7)));")
            .unwrap();

        let pattern = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("(((1,2),3),5);")
            .unwrap();

        let mut forest = BinForest::new(7);
        forest.add_tree(host);
        assert!(forest.isolate_tree(&pattern));

        // sort roots by the smallest leafs in them
        forest.roots.sort_by_cached_key(|c| {
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
            .parse_newick_from_str("(((1,2),(3,4)),(5,(6,7)));")
            .unwrap();

        let pattern = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,5),3);")
            .unwrap();

        let mut forest = BinForest::new(7);
        forest.add_tree(host);
        assert!(!forest.isolate_tree(&pattern));
    }
}
