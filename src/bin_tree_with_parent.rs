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

#[derive(Default)]
pub struct BinTreeWithParentBuilder {}

pub struct Node {
    parent: WeakNodeRef,
    depth: usize,
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

    fn new_inner(&mut self, left: Self::Node, right: Self::Node) -> Self::Node {
        let node_ref = Rc::new(RefCell::new(Node {
            parent: Weak::new(),
            depth: usize::MAX,
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
    pub fn top_down(&self) -> NodeCursor {
        self.clone()
    }

    pub fn update_topology(&self) {
        fn traverse(node: &NodeRef, depth: usize, parent: WeakNodeRef) {
            node.borrow_mut().depth = depth;
            node.borrow_mut().parent = parent;

            match &node.borrow().children {
                Children::Inner { left, right } => {
                    traverse(left, depth + 1, Rc::downgrade(node));
                    traverse(right, depth + 1, Rc::downgrade(node));
                }
                Children::Leaf { .. } => {}
            }
        }

        traverse(&self.0, 0, Weak::new());
    }
}

impl BottomUpCursor for NodeCursor {
    fn parent(&self) -> Option<Self> {
        Some(NodeCursor(self.0.borrow().parent.upgrade()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pace26io::{binary_tree::TopDownCursor, newick::BinaryTreeParser};

    #[test]
    fn newick_builder() {
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str("((1,2),3);")
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
            .parse_newick_from_str("((1,2),3);")
            .unwrap();

        let leaf = tree.top_down().left_child().unwrap().left_child().unwrap();
        assert_eq!(leaf.0.borrow().depth, 2);
        assert_eq!(leaf.parent().unwrap().0.borrow().depth, 1);
        assert_eq!(leaf.parent().unwrap().parent().unwrap().0.borrow().depth, 0);
        assert!(leaf.parent().unwrap().parent().unwrap().parent().is_none());
    }
}
