use crate::layout::{Direction, WindowNav};
use smithay::{
    desktop::Window,
    utils::{Logical, Rectangle},
};

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
                        let w1 = (rect.size.w as f32 * ratio) as i32;
                        (
                            Rectangle::new(rect.loc, (w1, rect.size.h).into()),
                            Rectangle::new(
                                (rect.loc.x + w1, rect.loc.y).into(),
                                (rect.size.w - w1, rect.size.h).into(),
                            ),
                        )
                    }
                    Axis::Vertical => {
                        let h1 = (rect.size.h as f32 * ratio) as i32;
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

    fn remove(&mut self, window: &Window) {
        todo!()
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
        todo!()
    }

    fn move_window(&mut self, dir: Direction) -> bool {
        todo!()
    }

    fn resize(&mut self, dir: Direction, delta: i32) -> bool {
        todo!()
    }
}
