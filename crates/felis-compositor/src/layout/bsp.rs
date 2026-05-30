use crate::layout::{Direction, WindowNav};
use smithay::desktop::Window;

type Node = usize;
pub struct BspLayout {
    nodes: Vec<Option<Node>>,
    free_list: Vec<usize>,
    root: Option<usize>,
    focused: Option<usize>,
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
}

impl WindowNav for BspLayout {
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
