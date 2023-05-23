use std::{cell::RefCell, rc::Rc};

use crate::state::CONFIG;
use smithay::{
    desktop::layer_map_for_output,
    utils::{Logical, Physical, Point, Rectangle, Size},
};
use tracing::debug;

use super::{
    binarytree::{BinaryTree, HorizontalOrVertical},
    workspace::{MagmaWindow, Workspace},
};

pub fn bsp_update_layout(workspace: &mut Workspace) {
    let gaps = CONFIG.gaps;
    //recalculate the size and location of the windows

    let output = layer_map_for_output(workspace.outputs().next().unwrap()).non_exclusive_zone();
    let output_full = workspace
        .outputs()
        .next()
        .unwrap()
        .current_mode()
        .unwrap()
        .size;

    match &mut workspace.layout_tree {
        BinaryTree::Empty => {}
        BinaryTree::Window(w) => {
            w.borrow_mut().rec = Rectangle {
                loc: Point::from((
                    gaps.0 + gaps.1 + output.loc.x,
                    gaps.0 + gaps.1 + output.loc.y,
                )),
                size: Size::from((
                    output.size.w - ((gaps.0 + gaps.1) * 2),
                    output.size.h - ((gaps.0 + gaps.1) * 2),
                )),
            };
        }
        BinaryTree::Split {
            left,
            right,
            split,
            ratio,
            counter_ratio,
        } => {
            if let BinaryTree::Window(w) = left.as_mut() {
                generate_layout(
                    right.as_mut(),
                    w,
                    Rectangle {
                        loc: Point::from((gaps.0 + output.loc.x, gaps.0 + output.loc.y)),
                        size: Size::from((
                            output.size.w - (gaps.0 * 2),
                            output.size.h - (gaps.0 * 2),
                        )),
                    },
                    *split,
                    *ratio,
                    Size::from((output_full.w - gaps.0, output_full.h - gaps.0)),
                    gaps,
                )
            }
        }
    }
    debug!("{:#?}", workspace.layout_tree);
    for magmawindow in workspace.magmawindows() {
        let xdg_toplevel = magmawindow.window.toplevel();
        xdg_toplevel.with_pending_state(|state| {
            state.size = Some(magmawindow.rec.size);
        });
        xdg_toplevel.send_configure();
    }
}

pub fn generate_layout(
    tree: &mut BinaryTree,
    lastwin: &Rc<RefCell<MagmaWindow>>,
    lastgeo: Rectangle<i32, Logical>,
    split: HorizontalOrVertical,
    ratio: f32,
    output: Size<i32, Physical>,
    gaps: (i32, i32),
) {
    let size = match split {
        HorizontalOrVertical::Horizontal => {
            Size::from(((lastgeo.size.w as f32 * ratio) as i32, lastgeo.size.h))
        }
        HorizontalOrVertical::Vertical => {
            Size::from((lastgeo.size.w, (lastgeo.size.h as f32 * ratio) as i32))
        }
    };

    let loc: Point<i32, Logical> = match split {
        HorizontalOrVertical::Horizontal => Point::from((lastgeo.loc.x, output.h - size.h)),
        HorizontalOrVertical::Vertical => Point::from((output.w - size.w, lastgeo.loc.y)),
    };

    let recgapped = Rectangle {
        size: Size::from((size.w - (gaps.1 * 2), (size.h - (gaps.1 * 2)))),
        loc: Point::from((loc.x + gaps.1, loc.y + gaps.1)),
    };

    lastwin.borrow_mut().rec = recgapped;

    let loc = match split {
        HorizontalOrVertical::Horizontal => Point::from((output.w - size.w, lastgeo.loc.y)),
        HorizontalOrVertical::Vertical => Point::from((lastgeo.loc.x, output.h - size.h)),
    };

    let rec = Rectangle { size, loc };
    let recgapped = Rectangle {
        size: Size::from((size.w - (gaps.1 * 2), (size.h - (gaps.1 * 2)))),
        loc: Point::from((loc.x + gaps.1, loc.y + gaps.1)),
    };
    match tree {
        BinaryTree::Empty => {}
        BinaryTree::Window(w) => w.borrow_mut().rec = recgapped,
        BinaryTree::Split {
            split,
            ratio,
            counter_ratio,
            left,
            right,
        } => {
            if let BinaryTree::Window(w) = left.as_mut() {
                w.borrow_mut().rec = rec;
                generate_layout(right.as_mut(), w, rec, *split, *counter_ratio, output, gaps)
            }
        }
    }
}
