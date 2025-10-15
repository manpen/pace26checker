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
}

#[cfg(test)]
mod tests {
    use super::*;
    use pace26io::newick::BinaryTreeParser;

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
}
