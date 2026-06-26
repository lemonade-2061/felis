use crate::layout::{Direction, WindowNav};
use smithay::desktop::Window;

type NodeId = usize;

enum Axis {
    Horizontal,
    Vertical,
}

enum NodeKind {
    Leaf {
        window: Window,
    },

    Split {
        axis: Axis,
        ratio: f32,
        first: NodeId,
        second: NodeId,
    },
}

struct BspNode {
    kind: NodeKind,
    parent: Option<NodeId>,
}

struct BspLayout {
    nodes: Vec<Option<BspNode>>,
    free_list: Vec<NodeId>,
    root: Option<NodeId>,
    focused: Option<NodeId>,
}

impl BspLayout {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_list: Vec::new(),
            root: None,
            focused: None,
        }
    }

    fn alloc(&mut self, kind: NodeKind) -> NodeId {
        let node = BspNode { kind, parent: None };

        match self.free_list.pop() {
            Some(id) => {
                self.nodes[id] = Some(node);
                id
            }

            None => {
                self.nodes.push(Some(node));
                self.nodes.len() - 1
            }
        }
    }
}

impl WindowNav for BspLayout {
    fn add(&mut self, window: Window) {
        let new_leaf = self.alloc(NodeKind::Leaf { window });

        match self.focused {
            None => {
                self.root = Some(new_leaf);
            }
            Some(target) => {
                let window = match &self.nodes[target] {
                    Some(BspNode {
                        kind: NodeKind::Leaf { window },
                        ..
                    }) => window.clone(),
                    _ => unreachable!("focusedは葉のはず"),
                };

                let old_leaf = self.alloc(NodeKind::Leaf { window });
                let node = self.nodes[target].as_mut().unwrap();

                node.kind = NodeKind::Split {
                    axis: Axis::Horizontal,
                    ratio: 0.5,
                    first: old_leaf,
                    second: new_leaf,
                };

                self.nodes[old_leaf].as_mut().unwrap().parent = Some(target);
                self.nodes[new_leaf].as_mut().unwrap().parent = Some(target);
            }
        }
        self.focused = Some(new_leaf);
    }
    fn focus(&mut self, dir: Direction) -> Option<Window> {
        todo!()
    }

    fn move_window(&mut self, dir: Direction) -> bool {
        todo!()
    }

    fn resize(&mut self, dir: Direction, delta: i32) -> bool {
        todo!()
    }
}
