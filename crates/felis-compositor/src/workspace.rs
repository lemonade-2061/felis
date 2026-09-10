use smithay::{
    desktop::Window,
    utils::{Logical, Rectangle},
};

use crate::layout::{bsp::BspLayout, floating::FloatingLayout, WindowNav, Direction};

pub struct Workspace {
    tiling: BspLayout,
    floating: FloatingLayout,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            tiling: BspLayout::new(),
            floating: FloatingLayout::new(),
        }
    }

    pub fn add_tiled(&mut self, window: Window) {
        self.tiling.add(window);
    }

    pub fn add_floating(&mut self, window: Window) {
        self.floating.add(window);
    }

    pub fn remove(&mut self, window: &Window) {
        self.tiling.remove(window);
        self.floating.remove(window);
    }

    pub fn layout(&self, area: Rectangle<i32, Logical>) -> Vec<(Window, Rectangle<i32, Logical>)> {
        self.tiling.layout(area)
    }

    pub fn floating_windows(&self) -> Vec<Window> {
        self.floating.windows()
    }

    pub fn focus(&mut self, dir: Direction) -> Option<Window> {
        self.tiling.focus(dir)
    }

    pub fn move_window(&mut self, dir: Direction) -> bool {
        self.tiling.move_window(dir)
    }

    pub fn set_focus(&mut self, window: &Window) {
        self.tiling.focus_window(window);
    }
}

