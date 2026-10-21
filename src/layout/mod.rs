use smithay::{
    desktop::Window,
    utils::{Logical, Rectangle},
};
pub mod bsp;
pub mod floating;
pub mod scroll;

#[derive(Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub trait WindowNav {
    fn add(&mut self, window: Window);
    fn remove(&mut self, window: &Window);
    /// レイアウト切り替え時に全ウィンドウを取り出して新レイアウトへ移すために使う
    fn windows(&self) -> Vec<Window>;
    fn layout(&self, area: Rectangle<i32, Logical>) -> Vec<(Window, Rectangle<i32, Logical>)>;
    fn focus(&mut self, dir: Direction) -> Option<Window>;
    fn move_window(&mut self, dir: Direction) -> bool;
    fn resize(&mut self, dir: Direction, delta: i32) -> bool;
}
