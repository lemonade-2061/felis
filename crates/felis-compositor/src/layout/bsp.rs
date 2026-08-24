use crate::layout::{Direction, WindowNav};
use smithay::{
    desktop::Window,
    utils::{Logical, Rectangle},
};

type NodeId = usize;

#[derive(Clone, Copy)]
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

pub struct BspLayout {
    nodes: Vec<Option<BspNode>>,
    free_list: Vec<NodeId>,
    root: Option<NodeId>,
    focused: Option<NodeId>,
}

impl BspLayout {
    pub fn new() -> Self {
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

    fn geometry(
        &self,
        node: NodeId,
        rect: Rectangle<i32, Logical>,
        out: &mut Vec<(Window, Rectangle<i32, Logical>)>,
    ) {
        match &self.nodes[node] {
            Some(BspNode {
                kind: NodeKind::Leaf { window },
                ..
            }) => {
                out.push((window.clone(), rect));
            }

            Some(BspNode {
                kind:
                    NodeKind::Split {
                        axis,
                        ratio,
                        first,
                        second,
                    },
                ..
            }) => {
                let (r1, r2) = match axis {
                    Axis::Horizontal => {
                        let w1 = ((rect.size.w as f32 * ratio) as i32).clamp(1, rect.size.w - 1);
                        (
                            Rectangle::new(rect.loc, (w1, rect.size.h).into()),
                            Rectangle::new(
                                (rect.loc.x + w1, rect.loc.y).into(),
                                (rect.size.w - w1, rect.size.h).into(),
                            ),
                        )
                    }
                    Axis::Vertical => {
                        let h1 = ((rect.size.h as f32 * ratio) as i32).clamp(1, rect.size.h - 1);
                        (
                            Rectangle::new(rect.loc, (rect.size.w, h1).into()),
                            Rectangle::new(
                                (rect.loc.x, rect.loc.y + h1).into(),
                                (rect.size.w, rect.size.h - h1).into(),
                            ),
                        )
                    }
                };
                self.geometry(*first, r1, out);
                self.geometry(*second, r2, out);
            }

            None => {}
        }
    }

    fn free(&mut self, id: NodeId) {
        self.nodes[id] = None;
        self.free_list.push(id);
    }

    fn find_leaf(&self, window: &Window) -> Option<NodeId> {
        self.nodes.iter().position(|slot| {
            matches!(
                slot,
                Some(BspNode {
                    kind: NodeKind::Leaf { window: w },
                    ..
                }) if w == window
            )
        })
    }

    fn first_leaf(&self, mut id: NodeId) -> NodeId {
        loop {
            match &self.nodes[id] {
                Some(BspNode {
                    kind: NodeKind::Split { first, .. },
                    ..
                }) => id = *first,
                _ => return id,
            }
        }
    }

    fn edge_leaf(&self, mut id: NodeId, dir: Direction) -> NodeId {
        loop {
            match &self.nodes[id].as_ref().unwrap().kind {
                NodeKind::Split {
                    axis, first, second, ..
                } => {
                    id = match (dir, axis) {
                        (Direction::Right, Axis::Horizontal) => *first,
                        (Direction::Left, Axis::Horizontal) => *second,
                        (Direction::Down, Axis::Vertical) => *first,
                        (Direction::Up, Axis::Vertical) => *second,
                        _ => *first,
                    };
                }
                _ => return id,
            }
        }
    }

    pub fn focus_window(&mut self, window: &Window) {
        if let Some(leaf) = self.find_leaf(window) {
            self.focused = Some(leaf);
        }
    }

    fn collect_windows(&self, node: NodeId, out: &mut Vec<Window>) {
        match &self.nodes[node] {
            Some(BspNode {
                kind: NodeKind::Leaf { window },
                ..
            }) => out.push(window.clone()),

            Some(BspNode {
                kind: NodeKind::Split { first, second, .. },
                ..
            }) => {
                self.collect_windows(*first, out);
                self.collect_windows(*second, out);
            }

            None => {}
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

                let axis = match self.nodes[target].as_ref().unwrap().parent {
                    None => Axis::Horizontal,
                    Some(p) => match &self.nodes[p].as_ref().unwrap().kind {
                        NodeKind::Split {
                            axis: Axis::Horizontal,
                            ..
                        } => Axis::Vertical,
                        _ => Axis::Horizontal,
                    },
                };

                let old_leaf = self.alloc(NodeKind::Leaf { window });
                let node = self.nodes[target].as_mut().unwrap();

                node.kind = NodeKind::Split {
                    axis,
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

    fn remove(&mut self, window: &Window) {
        let Some(leaf) = self.find_leaf(window) else {
            return;
        };

        let Some(parent) = self.nodes[leaf].as_ref().unwrap().parent else {
            self.free(leaf);
            self.root = None;
            self.focused = None;
            return;
        };

        let (first, second) = match &self.nodes[parent] {
            Some(BspNode {
                kind: NodeKind::Split { first, second, .. },
                ..
            }) => (*first, *second),
            _ => unreachable!("葉の親はSplitのはず"),
        };
        let sibling = if first == leaf { second } else { first };
        let grandparent = self.nodes[parent].as_ref().unwrap().parent;

        self.nodes[sibling].as_mut().unwrap().parent = grandparent;
        match grandparent {
            None => self.root = Some(sibling),
            Some(gp) => match &mut self.nodes[gp].as_mut().unwrap().kind {
                NodeKind::Split { first, second, .. } => {
                    if *first == parent {
                        *first = sibling;
                    } else {
                        *second = sibling;
                    }
                }
                _ => unreachable!("祖父母はSplitのはず"),
            },
        }

        self.free(leaf);
        self.free(parent);

        if self.focused == Some(leaf) {
            self.focused = Some(self.first_leaf(sibling));
        }
    }

    fn windows(&self) -> Vec<Window> {
        let mut out = Vec::new();
        if let Some(root) = self.root {
            self.collect_windows(root, &mut out);
        }
        out
    }

    fn layout(&self, area: Rectangle<i32, Logical>) -> Vec<(Window, Rectangle<i32, Logical>)> {
        let mut out = Vec::new();
        if let Some(root) = self.root {
            self.geometry(root, area, &mut out);
        }
        out
    }

    fn focus(&mut self, dir: Direction) -> Option<Window> {
        let mut cur = self.focused?;

        loop {
            let parent = self.nodes[cur].as_ref().unwrap().parent?;

            let(axis, first, second) = match &self.nodes[parent].as_ref().unwrap().kind {
                NodeKind::Split {
                    axis, first, second, ..
                } => (*axis, *first, *second),
                _ => unreachable!("parent is split")
            };

            let target =  match (dir, axis) {
                (Direction::Right, Axis::Horizontal) | (Direction::Down, Axis::Vertical)
                    if cur == first =>
                {
                    Some(second)
                }
                (Direction::Left, Axis::Horizontal) | (Direction::Up, Axis::Vertical)
                    if cur == second =>
                {
                    Some(first)
                }
                _ => None,
            };

            if let Some(target) = target {
                let leaf = self.edge_leaf(target, dir);
                self.focused = Some(leaf);
                return match &self.nodes[leaf].as_ref().unwrap().kind {
                    NodeKind::Leaf { window } => Some(window.clone()),
                    _ => unreachable!("edge_leaf return leaf")
                };
            }

            cur = parent;
        }
    }

    fn move_window(&mut self, dir: Direction) -> bool {
        todo!()
    }

    fn resize(&mut self, dir: Direction, delta: i32) -> bool {
        todo!()
    }
}
