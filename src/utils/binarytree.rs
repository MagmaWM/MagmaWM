use smithay::desktop::Window;
use std::fmt::Debug;
use std::{cell::RefCell, rc::Rc};

use super::workspace::MagmaWindow;

#[derive(Clone)]
pub enum BinaryTree {
    Empty,
    Window(Rc<RefCell<MagmaWindow>>),
    Split {
        split: HorizontalOrVertical,
        ratio: f32,
        counter_ratio: f32,
        left: Box<BinaryTree>,
        right: Box<BinaryTree>,
    },
}

impl Debug for BinaryTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Window(w) => w.borrow().rec.fmt(f),
            Self::Split {
                left,
                right,
                split,
                ratio,
                counter_ratio,
            } => f
                .debug_struct("Split")
                .field("split", split)
                .field("ratio", ratio)
                .field("left", left)
                .field("right", right)
                .field("counter-ratio", counter_ratio)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizontalOrVertical {
    Horizontal,
    Vertical,
}

impl BinaryTree {
    pub fn new() -> Self {
        BinaryTree::Empty
    }

    pub fn insert(
        &mut self,
        window: Rc<RefCell<MagmaWindow>>,
        splitnew: HorizontalOrVertical,
        rationew: f32,
    ) {
        match self {
            BinaryTree::Empty => {
                *self = BinaryTree::Window(window);
            }
            BinaryTree::Window(w) => {
                let counter_rationew = 1.0f32 - rationew;
                *self = BinaryTree::Split {
                    left: Box::new(BinaryTree::Window(w.clone())),
                    right: Box::new(BinaryTree::Window(window)),
                    split: splitnew,
                    ratio: rationew,
                    counter_ratio: counter_rationew,
                };
            }
            BinaryTree::Split {
                left: _,
                right,
                split: _,
                ratio: _,
                counter_ratio: _,
            } => {
                right.insert(window, splitnew, rationew);
            }
        }
    }

    pub fn remove(&mut self, window: &Window) {
        match self {
            BinaryTree::Empty => {}
            BinaryTree::Window(w) => {
                // Should only happen if this is the root
                if w.borrow().window == *window {
                    *self = BinaryTree::Empty;
                }
            }
            BinaryTree::Split {
                left,
                right,
                split: _,
                ratio: _,
                counter_ratio: _,
            } => {
                if let BinaryTree::Window(w) = left.as_ref() {
                    if w.borrow().window == *window {
                        *self = *right.clone();
                        return;
                    }
                }
                if let BinaryTree::Window(w) = right.as_ref() {
                    if w.borrow().window == *window {
                        *self = *left.clone();
                        return;
                    }
                }
                left.remove(window);
                right.remove(window);
            }
        }
    }

    pub fn next_split(&self) -> HorizontalOrVertical {
        match self {
            BinaryTree::Empty => HorizontalOrVertical::Horizontal,
            BinaryTree::Window(_w) => HorizontalOrVertical::Horizontal,
            BinaryTree::Split {
                left: _,
                right,
                split,
                ratio: _,
                counter_ratio: _,
            } => {
                if let BinaryTree::Split {
                    left: _,
                    right: _,
                    split: _,
                    ratio: _,
                    counter_ratio: _,
                } = right.as_ref()
                {
                    right.next_split()
                } else if *split == HorizontalOrVertical::Horizontal {
                    HorizontalOrVertical::Vertical
                } else {
                    HorizontalOrVertical::Horizontal
                }
            }
        }
    }

    // Updates the size ratio of split nodes
    // If increment is set to `None`, it will completely change the ratio to `new-ratio`
    // If increment is `Some` and set to true, the ratio will be incremented by the `new_ratio`
    // If increment is `Some` and set to false, the ratio will be decremented by the `new_ratio`
    pub fn update_ratio(&mut self, new_ratio: f32, increment: Option<bool>) {
        match self {
            BinaryTree::Empty => {}
            BinaryTree::Window(_) => {}
            BinaryTree::Split {
                split: _,
                ratio,
                counter_ratio,
                left: _,
                right,
            } => {
                match increment {
                    Some(increment) => {
                        if increment {
                            *ratio += new_ratio;
                        } else {
                            *ratio -= new_ratio;
                        }
                    }
                    None => {
                        *ratio = new_ratio;
                    }
                }
                *counter_ratio = 1.0f32 - *ratio;

                /*if let BinaryTree::Split {
                    split: _,
                    ratio: _,
                    left: _,
                    right: _,
                    counter_ratio: _,
                } = right.as_ref()
                {
                    if increment.is_some() {
                        right.update_ratio(new_ratio, Some(!increment.unwrap()));
                    } else {
                        right.update_ratio(new_ratio, increment);
                    }
                };*/
            }
        }
    }
}

impl Default for BinaryTree {
    fn default() -> Self {
        Self::new()
    }
}
