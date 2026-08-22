use smithay::{
    desktop::Window,
    utils::{Logical, Rectangle},
};

use crate::layout::{bsp::BspLayout, floating::FloatingLayout, WindowNav};

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
}

