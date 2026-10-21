use crate::layout::{Direction, WindowNav};
use smithay::{
    desktop::Window,
    utils::{Logical, Rectangle},
};
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
    fn add(&mut self, window: Window) {
        self.windows.push(window);
        self.focused = Some(self.windows.len() - 1);
    }

    fn focus(&mut self, dir: Direction) -> Option<Window> {
        let cur = self.focused?;

        let next = match dir {
            Direction::Right | Direction::Down => cur + 1,
            Direction::Left | Direction::Up => cur.checked_sub(1)?,
        };

        if next >= self.windows.len() {
            self.focused = Some(0);
            return Some(self.windows[0].clone());
        }

        self.focused = Some(next);
        Some(self.windows[next].clone())
    }

    fn remove(&mut self, window: &Window) {
        if let Some(pos) = self.windows.iter().position(|w| w == window) {
            self.windows.remove(pos);
            self.focused = if self.windows.is_empty() {
                None
            } else {
                Some(pos.min(self.windows.len() - 1))
            };
        }
    }

    fn windows(&self) -> Vec<Window> {
        self.windows.clone()
    }

    fn layout(&self, _area: Rectangle<i32, Logical>) -> Vec<(Window, Rectangle<i32, Logical>)> {
        // フローティングの座標は各ウィンドウが保持する想定。
        // レイヤー化(Workspace側でVec<(Window, Rectangle)>を持つ)の際にここを実装する。
        Vec::new()
    }

    fn move_window(&mut self, _dir: super::Direction) -> bool {
        false
    }

    fn resize(&mut self, _dir: super::Direction, _delta: i32) -> bool {
        false
    }
}
