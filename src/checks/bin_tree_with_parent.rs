use pace26io::binary_tree::*;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub trait BottomUpCursor: Sized {
    fn parent(&self) -> Option<Self>;
}

#[derive(Clone)]
pub struct NodeCursor(NodeRef);

#[derive(Clone, Default)]
pub struct WeakNodeCursor(WeakNodeRef);

#[derive(Default)]
pub struct BinTreeWithParentBuilder {}

pub struct Node {
    parent: WeakNodeRef,
    depth: usize,
    id: NodeIdx,
    children: Children,
}

enum Children {
    Inner { left: NodeRef, right: NodeRef },
    Leaf { label: Label },
}

type NodeRef = Rc<RefCell<Node>>;
type WeakNodeRef = Weak<RefCell<Node>>;

impl TreeBuilder for BinTreeWithParentBuilder {
    type Node = NodeCursor;

    fn new_inner(&mut self, id: NodeIdx, left: Self::Node, right: Self::Node) -> Self::Node {
        let node_ref = Rc::new(RefCell::new(Node {
            parent: Weak::new(),
            depth: usize::MAX,
            id,
            children: Children::Inner {
                left: left.0,
                right: right.0,
            },
        }));

        if let Children::Inner { left, right } = &node_ref.borrow().children {
            left.borrow_mut().parent = Rc::downgrade(&node_ref);
            right.borrow_mut().parent = Rc::downgrade(&node_ref);
        } else {
            unreachable!();
        }

        NodeCursor(node_ref)
    }

    fn new_leaf(&mut self, label: Label) -> Self::Node {
        NodeCursor(Rc::new(RefCell::new(Node {
            parent: Weak::new(),
            id: label.into(),
            children: Children::Leaf { label },
            depth: usize::MAX,
        })))
    }

    fn make_root(&mut self, root: Self::Node) -> Self::Node {
        root.update_topology();
        root
    }
}

impl TopDownCursor for NodeCursor {
    fn children(&self) -> Option<(Self, Self)> {
        match &self.0.borrow().children {
            Children::Inner { left, right } => {
                Some((NodeCursor(left.clone()), NodeCursor(right.clone())))
            }
            Children::Leaf { .. } => None,
        }
    }

    fn leaf_label(&self) -> Option<Label> {
        match &self.0.borrow().children {
            Children::Leaf { label } => Some(*label),
            Children::Inner { .. } => None,
        }
    }
}

impl NodeCursor {
    pub fn depth(&self) -> usize {
        self.0.borrow().depth
    }

    pub fn top_down(&self) -> NodeCursor {
        self.clone()
    }

    pub fn downgrade(&self) -> WeakNodeCursor {
        WeakNodeCursor(Rc::downgrade(&self.0))
    }

    /// Corrects depth and parent information in all nodes assuming `self` is the trees root.
    pub fn update_topology(&self) {
        Self::update_topology_internal(&self.0, 0, Weak::new());
    }

    /// Similar to [`NodeCursor::update_topology`] but intended for subtrees. It is assumed that
    /// `parent` and `depth` are valid for `self`.
    pub fn update_topology_subtree(&self) {
        let parent = self.0.borrow().parent.clone();
        Self::update_topology_internal(&self.0, self.depth(), parent);
    }

    fn update_topology_internal(node: &NodeRef, depth: usize, parent: WeakNodeRef) {
        node.borrow_mut().depth = depth;
        node.borrow_mut().parent = parent;

        match &node.borrow().children {
            Children::Inner { left, right } => {
                Self::update_topology_internal(left, depth + 1, Rc::downgrade(node));
                Self::update_topology_internal(right, depth + 1, Rc::downgrade(node));
            }
            Children::Leaf { .. } => {}
        }
    }

    pub fn lowest_common_ancestor(mut a: Self, mut b: Self) -> Option<NodeCursor> {
        // climb until both nodes are at the same depth
        {
            if a.depth() < b.depth() {
                std::mem::swap(&mut a, &mut b);
            }
            // invariant: a.depth() >= b.depth()

            while a.depth() > b.depth() {
                a = a.parent()?;
            }

            assert_eq!(a.depth(), b.depth());
        }

        while a != b {
            a = a.parent()?;
            b = b.parent()?;
        }

        Some(a)
    }

    pub fn replace_child(&self, old: NodeCursor, new: NodeCursor) {
        debug_assert!(old.parent().as_ref().is_some_and(|p| p == self));

        let (left, right) = self.children().unwrap();
        new.0.borrow_mut().parent = self.downgrade().0;

        self.0.borrow_mut().children = if left == old {
            Children::Inner {
                left: new.0,
                right: right.0,
            }
        } else {
            Children::Inner {
                left: left.0,
                right: new.0,
            }
        }
    }

    pub fn sibling(&self) -> Option<NodeCursor> {
        let parent = self.parent()?;
        let (left, right) = parent.children().unwrap();

        Some(if self == &left {
            right
        } else {
            debug_assert!(self == &right);
            left
        })
    }

    pub fn remove_sibling(&self) -> Option<NodeCursor> {
        let parent = self.parent()?;

        let parent_parent = parent.parent()?;
        let sibling = self.sibling()?;

        parent_parent.replace_child(parent, self.clone());

        Some(sibling)
    }
}

impl WeakNodeCursor {
    pub fn upgrade(&self) -> Option<NodeCursor> {
        Some(NodeCursor(self.0.upgrade()?))
    }
}

impl BottomUpCursor for NodeCursor {
    fn parent(&self) -> Option<Self> {
        Some(NodeCursor(self.0.borrow().parent.upgrade()?))
    }
}

impl PartialEq for NodeCursor {
    /// Two NodeCursor are equal if they point to the same node address, indiscriminantly
    /// of the values stored there
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for NodeCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((l, r)) = self.children() {
            write!(f, "({:?},{:?})", l, r)
        } else if let Some(l) = self.leaf_label() {
            write!(f, "{}", l.0)
        } else {
            unreachable!();
        }
    }
}

impl TreeWithNodeIdx for NodeCursor {
    fn node_idx(&self) -> NodeIdx {
        self.0.borrow().id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pace26io::{binary_tree::TopDownCursor, newick::BinaryTreeParser};

    fn get_leaf(tree: &NodeCursor, label: u32) -> NodeCursor {
        tree.top_down()
            .dfs()
            .find(|l| l.leaf_label() == Some(Label(label)))
            .unwrap()
    }

    #[test]
    fn newick_builder() {
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,2),3);", Default::default())
            .unwrap();

        assert_eq!(tree.0.borrow().depth, 0);
        assert_eq!(tree.top_down().left_child().unwrap().0.borrow().depth, 1);
        assert_eq!(
            tree.top_down()
                .left_child()
                .unwrap()
                .left_child()
                .unwrap()
                .0
                .borrow()
                .depth,
            2
        );
        assert_eq!(
            tree.top_down()
                .left_child()
                .unwrap()
                .right_child()
                .unwrap()
                .0
                .borrow()
                .depth,
            2
        );
        assert_eq!(tree.top_down().right_child().unwrap().0.borrow().depth, 1);
    }

    #[test]
    fn bottom_up_cursor() {
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,2),3);", Default::default())
            .unwrap();

        let leaf = tree.top_down().left_child().unwrap().left_child().unwrap();
        assert_eq!(leaf.0.borrow().depth, 2);
        assert_eq!(leaf.parent().unwrap().0.borrow().depth, 1);
        assert_eq!(leaf.parent().unwrap().parent().unwrap().0.borrow().depth, 0);
        assert!(leaf.parent().unwrap().parent().unwrap().parent().is_none());
    }

    #[test]
    fn lowest_common_ancestor() {
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,2),(3,(4,5)));", Default::default())
            .unwrap();

        fn lca_depth(a: &NodeCursor, b: &NodeCursor) -> Option<usize> {
            NodeCursor::lowest_common_ancestor(a.clone(), b.clone()).map(|lca| lca.depth())
        }

        let leaf1 = get_leaf(&tree, 1);
        let leaf2 = get_leaf(&tree, 2);
        let leaf3 = get_leaf(&tree, 3);
        let leaf4 = get_leaf(&tree, 4);
        let leaf5 = get_leaf(&tree, 5);

        assert_eq!(leaf1.depth(), 2);
        assert_eq!(lca_depth(&leaf1, &leaf1), Some(2));
        assert_eq!(lca_depth(&leaf1, &leaf2), Some(1));
        assert_eq!(lca_depth(&leaf1, &leaf3), Some(0));
        assert_eq!(lca_depth(&leaf4, &leaf4), Some(3));
        assert_eq!(lca_depth(&leaf4, &leaf5), Some(2));

        let tree2 = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,2),(3,(4,5)));", Default::default())
            .unwrap();

        let leaf1_in_tree2 = get_leaf(&tree2, 1);
        assert!(lca_depth(&leaf1, &leaf1_in_tree2).is_none());
    }

    #[test]
    fn sibling() {
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,2),(3,(4,5)));", Default::default())
            .unwrap();

        assert_eq!(
            get_leaf(&tree, 1).sibling().unwrap().leaf_label(),
            Some(Label(2))
        );

        assert_eq!(
            get_leaf(&tree, 2).sibling().unwrap().leaf_label(),
            Some(Label(1))
        );

        assert_eq!(
            get_leaf(&tree, 3)
                .sibling()
                .unwrap()
                .left_child()
                .unwrap()
                .leaf_label(),
            Some(Label(4))
        );

        assert!(
            get_leaf(&tree, 1)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .sibling()
                .is_none()
        );
    }

    #[test]
    fn remove_sibling() {
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,2),3);", Default::default())
            .unwrap();

        let l1 = get_leaf(&tree, 1);
        assert_eq!(l1.depth(), 2);

        let l2 = l1.remove_sibling().unwrap();
        assert_eq!(l2.leaf_label(), Some(Label(2)));
        tree.update_topology();

        assert_eq!(l1.depth(), 1);
    }
}
