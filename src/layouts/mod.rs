use std::{sync::Arc, cell::RefCell};

use crate::utils::workspace::MagmaWindow;

pub mod dwindle;


pub trait WindowContainer {
    fn populate_container(&mut self, windows: &mut Vec<MagmaWindow>);
}

pub trait Layout {
    fn update_layout(&self, window_container: &mut Arc<RefCell<dyn WindowContainer>>);
}
