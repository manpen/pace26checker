use pace26io::newick::{BinaryTreeConstr, Label};
use std::{
    cell::{Ref, RefCell},
    rc::{Rc, Weak},
};

pub enum Children {
    Inner {
        left: Rc<RefCell<Node>>,
        right: Rc<RefCell<Node>>,
    },
    Leaf {
        label: Label,
    },
}

pub struct Node {
    parent: Weak<RefCell<Node>>,
    depth: usize,
    children: Children,
}

pub struct BinTree {
    root: Rc<RefCell<Node>>,
}

impl BinTree {
    pub fn update_topology(&mut self) {
        fn update(parent: Weak<RefCell<Node>>, node: &Rc<RefCell<Node>>, depth: usize) {
            {
                let mut mut_node = Rc::as_ref(node).borrow_mut();
                mut_node.parent = parent;
                mut_node.depth = depth + 1;
            }

            let weak_this = Rc::downgrade(node);
            match &node.borrow().children {
                Children::Inner { left, right } => {
                    update(weak_this.clone(), left, depth + 1);
                    update(weak_this, right, depth + 1);
                }
                Children::Leaf { .. } => {}
            }
        }

        update(Weak::new(), &self.root, 0);
    }

    
}

impl BinaryTreeConstr for Node {
    fn new_inner(left: Self, right: Self) -> Self {
        Self {
            parent: Weak::new(),
            depth: 0,
            children: Children::Inner {
                left: Rc::new(RefCell::new(left)),
                right: Rc::new(RefCell::new(right)),
            },
        }
    }

    fn new_leaf(label: Label) -> Self {
        Self {
            parent: Weak::new(),
            depth: 0,
            children: Children::Leaf { label },
        }
    }
}
