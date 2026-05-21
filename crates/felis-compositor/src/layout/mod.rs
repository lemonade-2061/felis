enum Direction {
    Right,
    Left
}
trait Horizontal {
    fn focus(&mut self, dir: Direction) -> option<window>
}
