use std::{sync::Arc, cell::RefCell};

use crate::utils::workspace::MagmaWindow;

use super::Layout;

enum Asymptote {
    Horizontal,
    Vertical
}

enum Node {
    Empty,
    Window(Arc<RefCell<MagmaWindow>>),
    Split {
        left: Arc<RefCell<Node>>,
        right: Arc<RefCell<Node>>,
        asymp: Asymptote,
        ratio: f32
    }
}

pub struct BinaryTree {

}

pub struct DwindleLayout {
    in_use: bool,
}

impl DwindleLayout {
    pub fn new(activate: bool) -> Self {
        DwindleLayout { in_use: activate }
    }
}

impl Layout for DwindleLayout {

    fn update_layout(&self, window_container: &mut std::sync::Arc<std::cell::RefCell<dyn super::WindowContainer>>) {
        todo!()
    }
}