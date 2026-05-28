use smithay::desktop::Window;
pub mod bsp;
pub mod floating;
pub mod scroll;
enum Direction {
    Right,
    Left,
}
trait Horizontal {
    fn focus(&mut self, dir: Direction) -> Option<Window>;
    fn move_window(&mut self, dir: Direction) -> bool;
    fn resize(&mut self, dir: Direction, delta: i32) -> bool;
}
