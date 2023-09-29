use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
        gles::element::PixelShaderElement,
        ImportAll, Renderer, Texture,
    },
    desktop::{space::SpaceElement, Window},
    output::Output,
    utils::{Logical, Point, Rectangle, Scale, Transform},
    wayland::{compositor::with_states, shell::xdg::SurfaceCachedState},
};

use crate::state::CONFIG;

use super::{
    binarytree::BinaryTree,
    render::{border::BorderShader, AsGlowRenderer},
    tiling::bsp_update_layout,
};

#[derive(Debug, PartialEq, Clone)]
pub struct MagmaWindow {
    pub window: Window,
    pub rec: Rectangle<i32, Logical>,
}
impl MagmaWindow {
    fn bbox(&self) -> Rectangle<i32, Logical> {
        let mut bbox = self.window.bbox();
        bbox.loc += self.rec.loc - self.window.geometry().loc;
        bbox
    }

    fn render_location(&self) -> Point<i32, Logical> {
        self.rec.loc - self.window.geometry().loc
    }
}
pub struct Workspace {
    windows: Vec<Rc<RefCell<MagmaWindow>>>,
    outputs: Vec<Output>,
    pub layout_tree: BinaryTree,
    pub floating: Vec<Rc<RefCell<MagmaWindow>>>,
}

impl Workspace {
    pub fn new() -> Self {
        Workspace {
            windows: Vec::new(),
            outputs: Vec::new(),
            layout_tree: BinaryTree::new(),
            floating: Vec::new(),
        }
    }

    pub fn windows(&self) -> impl Iterator<Item = Ref<'_, Window>> {
        self.windows
            .iter()
            .map(|w| Ref::map(w.borrow(), |hw| &hw.window))
    }

    pub fn magmawindows(&self) -> impl Iterator<Item = Ref<'_, MagmaWindow>> {
        self.windows.iter().map(|w| Ref::map(w.borrow(), |hw| hw))
    }

    pub fn add_window(&mut self, window: Rc<RefCell<MagmaWindow>>) {
        // add window to vec and remap if exists
        self.windows
            .retain(|w| w.borrow().window != window.borrow().window);
        self.windows.insert(0, window.clone());
        let (max_size, min_size) =
            with_states(window.borrow().window.toplevel().wl_surface(), |states| {
                let attr = states.cached_state.current::<SurfaceCachedState>();
                dbg!(attr.max_size, attr.min_size)
            });
        let parent = dbg!(window.borrow().window.toplevel().parent().is_some());
        if (min_size.w != 0
            && min_size.h != 0
            && (min_size.w == max_size.w || min_size.h == max_size.h))
            || parent
        {
            let mw = window.borrow().rec;
            let os = self
                .outputs
                .first()
                .unwrap()
                .current_mode()
                .unwrap()
                .size
                .to_logical(1);
            window.borrow_mut().rec.loc =
                Point::from((os.w / 2 - mw.size.w / 2, os.h / 2 - mw.size.h / 2));
            self.floating.push(window);
        } else {
            self.layout_tree
                .insert(window, self.layout_tree.next_split(), 0.5);
            bsp_update_layout(self);
        }
    }

    pub fn remove_window(&mut self, window: &Window) -> Option<Rc<RefCell<MagmaWindow>>> {
        let mut removed = None;
        self.windows.retain(|w| {
            if &w.borrow().window == window {
                removed = Some(w.clone());
                false
            } else {
                true
            }
        });
        self.layout_tree.remove(window);
        bsp_update_layout(self);
        removed
    }

    pub fn render_elements<
        R: Renderer + ImportAll + AsGlowRenderer,
        C: From<WaylandSurfaceRenderElement<R>> + From<PixelShaderElement>,
    >(
        &self,
        renderer: &mut R,
    ) -> Vec<C>
    where
        <R as Renderer>::TextureId: Texture + 'static,
    {
        let mut render_elements: Vec<C> = Vec::new();
        for element in &self.windows {
            let window = &element.borrow().window;
            if CONFIG.borders.thickness > 0 {
                render_elements.push(C::from(BorderShader::element(
                    renderer.glow_renderer_mut(),
                    window,
                    element.borrow().rec.loc,
                )));
            }
            render_elements.append(&mut window.render_elements(
                renderer,
                element.borrow().render_location().to_physical(1),
                Scale::from(1.0),
                1.0,
            ));
        }
        render_elements
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.outputs.iter()
    }

    pub fn add_output(&mut self, output: Output) {
        self.outputs.push(output);
    }

    pub fn remove_output(&mut self, output: &Output) {
        self.outputs.retain(|o| o != output);
    }

    pub fn output_geometry(&self, o: &Output) -> Option<Rectangle<i32, Logical>> {
        if !self.outputs.contains(o) {
            return None;
        }

        let transform: Transform = o.current_transform();
        o.current_mode().map(|mode| {
            Rectangle::from_loc_and_size(
                (0, 0),
                transform
                    .transform_size(mode.size)
                    .to_f64()
                    .to_logical(o.current_scale().fractional_scale())
                    .to_i32_ceil(),
            )
        })
    }

    pub fn window_under<P: Into<Point<f64, Logical>>>(
        &self,
        point: P,
    ) -> Option<(Window, Point<i32, Logical>)> {
        let point = point.into();
        let floating = self
            .floating
            .iter()
            .filter(|e| e.borrow().bbox().to_f64().contains(point))
            .find_map(|e| {
                // we need to offset the point to the location where the surface is actually drawn
                let render_location = e.borrow().render_location();
                if e.borrow()
                    .window
                    .is_in_input_region(&(point - render_location.to_f64()))
                {
                    Some((e.borrow().window.clone(), render_location))
                } else {
                    None
                }
            });

        let tree = self
            .layout_tree
            .get_windows()
            .iter()
            .filter(|e| e.borrow().bbox().to_f64().contains(point))
            .find_map(|e| {
                // we need to offset the point to the location where the surface is actually drawn
                let render_location = e.borrow().render_location();
                if e.borrow()
                    .window
                    .is_in_input_region(&(point - render_location.to_f64()))
                {
                    Some((e.borrow().window.clone(), render_location))
                } else {
                    None
                }
            });

        if let Some(floating) = floating {
            Some(floating)
        } else if let Some(tree) = tree {
            Some(tree)
        } else {
            None
        }
    }

    pub fn contains_window(&self, window: &Window) -> bool {
        self.windows.iter().any(|w| &w.borrow().window == window)
    }

    pub fn toggle_window_floating(&mut self, window: &Window) {
        if let Some(mwindow) = self
            .layout_tree
            .get_windows()
            .into_iter()
            .find(|w| &w.borrow().window == window)
        {
            self.layout_tree.remove(window);
            self.floating.push(mwindow);
        } else if let Some(mwindow) = self
            .floating
            .clone()
            .into_iter()
            .find(|w| &w.borrow().window == window)
        {
            self.floating.retain(|w| w != &mwindow);
            self.layout_tree
                .insert(mwindow, self.layout_tree.next_split(), 0.5)
        }
        bsp_update_layout(self)
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Workspaces {
    workspaces: Vec<Workspace>,
    pub current: u8,
    pub pending: Vec<Window>,
}

impl Workspaces {
    pub fn new(workspaceamount: u8) -> Self {
        Workspaces {
            workspaces: (0..workspaceamount).map(|_| Workspace::new()).collect(),
            current: 0,
            pending: Vec::new(),
        }
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.workspaces.iter().flat_map(|w| w.outputs())
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &mut Workspace> {
        self.workspaces.iter_mut()
    }

    pub fn current_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.current as usize]
    }

    pub fn current(&self) -> &Workspace {
        &self.workspaces[self.current as usize]
    }

    pub fn all_windows(&self) -> impl Iterator<Item = Ref<'_, Window>> {
        self.workspaces.iter().flat_map(|w| w.windows())
    }

    pub fn workspace_from_window(&mut self, window: &Window) -> Option<&mut Workspace> {
        self.workspaces
            .iter_mut()
            .find(|w| w.contains_window(window))
    }

    pub fn activate(&mut self, id: u8) {
        self.current = id;
    }
    pub fn move_window_to_workspace(&mut self, window: &Window, workspace: u8) {
        let mut removed = None;
        if let Some(ws) = self.workspace_from_window(window) {
            removed = ws.remove_window(window);
            bsp_update_layout(ws)
        }
        if let Some(removed) = removed {
            self.workspaces[workspace as usize].add_window(removed);
            bsp_update_layout(&mut self.workspaces[workspace as usize])
        }
    }
}
