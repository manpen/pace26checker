use crate::{
    checks::bin_forest::*,
    checks::bin_tree_with_parent::*,
    io::{instance_reader::Instance, solution_reader::Solution},
};
use pace26io::binary_tree::*;
use std::{collections::HashSet, io::Write};

/// Produce a visual representation of an instance (optionally together with a solution) in
/// GraphViz Dot format.
///
/// # Example
/// ```
/// use std::{path::PathBuf, io::stdout};
/// use pace26checker::{checks::checker::check_instance_and_solution, io::forest_dot_writer::*};
///
/// const PATH_INSTANCE: &str = "testcases/valid/score10_n07l_lkc.in";
/// const PATH_SOLUTION: &str = "testcases/valid/score10_n07l_lkc.out";
///
/// let (instance, solution, forests) = check_instance_and_solution(
///         &PathBuf::from(PATH_INSTANCE),
///         &PathBuf::from(PATH_SOLUTION),
///         false,
///         true,
///     )
///     .unwrap();
///
/// let mut writer = ForestDotWriter::new(instance.as_ref().unwrap());
/// writer.color_leafs(&solution, &forests);
/// writer.write(&mut stdout().lock()).unwrap();
/// ```
pub struct ForestDotWriter<'a> {
    instance: &'a Instance,
    colors: Vec<u32>,
    roots: Vec<HashSet<NodeIdx>>,
    leaf_names: Vec<String>,
}

impl<'a> ForestDotWriter<'a> {
    pub fn new(instance: &'a Instance) -> Self {
        let num_nodes =
            (1 + instance.num_trees() as usize) * (instance.num_leaves() as usize - 1) + 2;

        let colors = std::iter::repeat_n(1, num_nodes).collect();
        Self {
            instance,
            colors,
            leaf_names: Vec::with_capacity(
                instance.num_leaves() as usize * instance.num_trees() as usize,
            ),
            roots: Vec::with_capacity(instance.num_trees() as usize),
        }
    }

    pub fn color_leafs(&mut self, solution: &Solution, forests: &[BinForest]) {
        // color leaves using the index of the solution tree
        for (i, (_, tree)) in solution.trees().iter().enumerate() {
            for u in tree.clone().top_down().dfs() {
                if let Some(l) = u.leaf_label() {
                    self.colors[l.0 as usize] = 2 + i as u32;
                }
            }
        }

        for (forest, (_, inst)) in forests.iter().zip(self.instance.trees()) {
            let roots: HashSet<_> = forest.roots().iter().map(|c| c.node_idx()).collect();

            fn recurse(
                colors: &mut [u32],
                roots: &HashSet<NodeIdx>,
                node: NodeCursor,
            ) -> (bool, u32) {
                let is_root = roots.contains(&node.node_idx());
                if let Some((left, right)) = node.children() {
                    let (l_root, l_color) = recurse(colors, roots, left);
                    let (r_root, r_color) = recurse(colors, roots, right);

                    let color = if l_color == r_color || !l_root {
                        l_color
                    } else if !r_root {
                        r_color
                    } else {
                        1
                    };

                    colors[node.node_idx().0 as usize] = color;
                }

                (is_root, colors[node.node_idx().0 as usize])
            }

            recurse(&mut self.colors, &roots, inst.clone());

            self.roots.push(roots);
        }
    }

    fn node_name(&self, name: &str, u: &NodeCursor) -> String {
        if let Some(l) = u.leaf_label() {
            format!("t{}l{}", name, l.0)
        } else {
            format!("t{}v{}", name, u.node_idx().0)
        }
    }

    fn recurse(
        &mut self,
        writer: &mut impl Write,
        root: NodeCursor,
        name: &str,
        roots: &HashSet<NodeIdx>,
    ) -> Result<(), std::io::Error> {
        let color = self.colors[root.node_idx().0 as usize];
        let my_key = self.node_name(name, &root);
        let is_root = roots.contains(&root.node_idx());

        fn can_reach_leaf(roots: &HashSet<NodeIdx>, node: &NodeCursor) -> bool {
            if roots.contains(&node.node_idx()) {
                false
            } else if let Some((l, r)) = node.children() {
                can_reach_leaf(roots, &l) || can_reach_leaf(roots, &r)
            } else {
                true
            }
        }

        if let Some((l, r)) = root.children() {
            let l_name = self.node_name(name, &l);
            let r_name = self.node_name(name, &r);
            let l_is_root = roots.contains(&l.node_idx());
            let r_is_root = roots.contains(&r.node_idx());

            let l_reaches_leaf = can_reach_leaf(roots, &l);
            let r_reaches_leaf = can_reach_leaf(roots, &r);

            writeln!(
                writer,
                "  {my_key}[label=\"{}\",color={color}{}]",
                root.node_idx().0,
                if is_root {
                    if l_reaches_leaf || r_reaches_leaf {
                        ",shape=\"triangle\""
                    } else {
                        ",style=\"dotted\""
                    }
                } else if l_reaches_leaf || r_reaches_leaf {
                    ""
                } else {
                    ",style=\"dotted\""
                }
            )?;

            writeln!(
                writer,
                "  {my_key} -> {l_name}{};",
                if l_is_root {
                    " [style=dashed]"
                } else if !l_reaches_leaf {
                    "[style=dotted]"
                } else {
                    ""
                },
            )?;
            self.recurse(writer, l, name, roots)?;

            writeln!(
                writer,
                "  {my_key} -> {r_name}{};",
                if r_is_root {
                    " [style=dashed]"
                } else if !r_reaches_leaf {
                    "[style=dotted]"
                } else {
                    ""
                },
            )?;
            self.recurse(writer, r, name, roots)?;
        } else if let Some(l) = root.leaf_label() {
            writeln!(
                writer,
                "  {my_key} [label=\"{}\", color={color}, shape=\"{}\"]",
                l.0,
                if is_root { "triangle" } else { "box" }
            )?;
            self.leaf_names.push(my_key);
        } else {
            unreachable!();
        }
        Ok(())
    }

    pub fn write(&mut self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        writeln!(writer, "digraph Instance {{")?;
        writeln!(writer, " rankdir=TB;")?;
        writeln!(writer, " node [colorscheme=set19];")?;

        for (i, (_lineno, tree)) in self.instance.trees().iter().enumerate() {
            let roots = self.roots.get(i).cloned().unwrap_or_default();
            let name = format!("t{}", i + 1);
            if i > 0 {
                writeln!(
                    writer,
                    " spacer{i} [shape=none, label=\"\", width=0, height=1];"
                )?;
            }

            writeln!(writer, "  subgraph {} {{", name)?;
            tree.normalize_child_order();
            self.recurse(writer, tree.clone(), &name, &roots)?;
            writeln!(writer, "  }}")?;
        }

        writeln!(writer, " {{rank=same;{}}}", self.leaf_names.join("; "))?;
        writeln!(writer, "}}")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::checks::checker::check_instance_and_solution;

    use super::*;

    const PATH_INSTANCE: &str = "testcases/valid/score10_n07l_lkc.in";
    const PATH_SOLUTION: &str = "testcases/valid/score10_n07l_lkc.out";

    #[test]
    fn instance_only() {
        let instance = Instance::read(&PathBuf::from(PATH_INSTANCE), false).unwrap();
        let mut writer = ForestDotWriter::new(&instance);
        let mut buffer: Vec<u8> = Vec::new();
        writer.write(&mut buffer).unwrap();
        String::from_utf8(buffer).unwrap();
    }

    #[test]
    fn instance_and_solution() {
        let (instance, solution, forests) = check_instance_and_solution(
            &PathBuf::from(PATH_INSTANCE),
            &PathBuf::from(PATH_SOLUTION),
            false,
            true,
        )
        .unwrap();

        let mut writer = ForestDotWriter::new(instance.as_ref().unwrap());
        writer.color_leafs(&solution, &forests);
        let mut buffer: Vec<u8> = Vec::new();
        writer.write(&mut buffer).unwrap();
        String::from_utf8(buffer).unwrap();
    }
}
