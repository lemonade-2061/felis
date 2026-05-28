use crate::layout::{Direction, WindowNav};
use smithay::desktop::Window;
pub struct FloatingLayout {
    windows: Vec<Window>,
    focused: Option<usize>,
}

impl FloatingLayout {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            focused: Option::None,
        }
    }
}

impl WindowNav for FloatingLayout {
    fn focus(&mut self, dir: Direction) -> Option<Window> {
        let cur = self.focused?;

        let next = match dir {
            Direction::Right => cur + 1,
            Direction::Left => cur.checked_sub(1)?,
        };

        if next >= self.windows.len() {
            return Some(self.windows[0].clone());
        }

        self.focused = Some(next);
        Some(self.windows[next].clone())
    }
    fn move_window(&mut self, _dir: super::Direction) -> bool {
        false
    }
    fn resize(&mut self, _dir: super::Direction, _delta: i32) -> bool {
        false
    }
}
