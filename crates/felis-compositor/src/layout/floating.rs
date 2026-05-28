use crate::layout::Horizontal;
use smithay::desktop::Window;
pub struct FloatingLayout {
    windows: Vec<Window>,
}

impl FloatingLayout {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }
}

impl Horizontal for FloatingLayout {
    fn focus(&mut self, dir: Direction) -> Option<Window> {
        let sign = match dir {
            Direction::Right => 1,
            Direction::Left => -1,
        };
    }
    fn move_window(&mut self, dir: super::Direction) -> bool {
        todo!()
    }
    fn resize(&mut self, dir: super::Direction, delta: i32) -> bool {
        todo!()
    }
}
