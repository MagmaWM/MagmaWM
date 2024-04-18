use std::{
    cell::{Ref, RefCell},
    hash::Hash,
    rc::Rc,
};

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
        gles::element::PixelShaderElement,
        ImportAll, Renderer, Texture,
    },
    desktop::{space::SpaceElement, Window},
    input::{keyboard::KeyboardTarget, pointer::PointerTarget, SeatHandler},
    output::Output,
    utils::{IsAlive, Logical, Point, Rectangle, Scale, Transform},
    wayland::seat::WaylandFocus,
    xwayland::X11Surface,
};

use crate::state::CONFIG;

use super::{
    binarytree::BinaryTree,
    render::{border::BorderShader, AsGlowRenderer},
    tiling::bsp_update_layout,
};

#[derive(Debug, PartialEq, Clone)]
pub enum WindowElement {
    Wayland(Window),
    X11(X11Surface),
}
impl std::cmp::Eq for WindowElement {}
impl Hash for WindowElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}
impl IsAlive for WindowElement {
    fn alive(&self) -> bool {
        match self {
            WindowElement::Wayland(w) => w.alive(),
            WindowElement::X11(x) => x.alive(),
        }
    }
}
impl<D: SeatHandler + 'static> PointerTarget<D> for WindowElement {
    fn enter(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => PointerTarget::enter(w, seat, data, event),
            WindowElement::X11(x) => PointerTarget::enter(x, seat, data, event),
        }
    }

    fn motion(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.motion(seat, data, event),
            WindowElement::X11(x) => x.motion(seat, data, event),
        }
    }

    fn relative_motion(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.relative_motion(seat, data, event),
            WindowElement::X11(x) => x.relative_motion(seat, data, event),
        }
    }

    fn button(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.button(seat, data, event),
            WindowElement::X11(x) => x.button(seat, data, event),
        }
    }

    fn axis(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        frame: smithay::input::pointer::AxisFrame,
    ) {
        match self {
            WindowElement::Wayland(w) => w.axis(seat, data, frame),
            WindowElement::X11(x) => x.axis(seat, data, frame),
        }
    }

    fn frame(&self, seat: &smithay::input::Seat<D>, data: &mut D) {
        match self {
            WindowElement::Wayland(w) => w.frame(seat, data),
            WindowElement::X11(x) => x.frame(seat, data),
        }
    }

    fn gesture_swipe_begin(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GestureSwipeBeginEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_swipe_begin(seat, data, event),
            WindowElement::X11(x) => x.gesture_swipe_begin(seat, data, event),
        }
    }

    fn gesture_swipe_update(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GestureSwipeUpdateEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_swipe_update(seat, data, event),
            WindowElement::X11(x) => x.gesture_swipe_update(seat, data, event),
        }
    }

    fn gesture_swipe_end(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GestureSwipeEndEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_swipe_end(seat, data, event),
            WindowElement::X11(x) => x.gesture_swipe_end(seat, data, event),
        }
    }

    fn gesture_pinch_begin(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GesturePinchBeginEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_pinch_begin(seat, data, event),
            WindowElement::X11(x) => x.gesture_pinch_begin(seat, data, event),
        }
    }

    fn gesture_pinch_update(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GesturePinchUpdateEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_pinch_update(seat, data, event),
            WindowElement::X11(x) => x.gesture_pinch_update(seat, data, event),
        }
    }

    fn gesture_pinch_end(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GesturePinchEndEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_pinch_end(seat, data, event),
            WindowElement::X11(x) => x.gesture_pinch_end(seat, data, event),
        }
    }

    fn gesture_hold_begin(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GestureHoldBeginEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_hold_begin(seat, data, event),
            WindowElement::X11(x) => x.gesture_hold_begin(seat, data, event),
        }
    }

    fn gesture_hold_end(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        event: &smithay::input::pointer::GestureHoldEndEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.gesture_hold_end(seat, data, event),
            WindowElement::X11(x) => x.gesture_hold_end(seat, data, event),
        }
    }

    fn leave(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        match self {
            WindowElement::Wayland(w) => PointerTarget::leave(w, seat, data, serial, time),
            WindowElement::X11(x) => PointerTarget::leave(x, seat, data, serial, time),
        }
    }
}

impl<D: SeatHandler + 'static> KeyboardTarget<D> for WindowElement {
    fn enter(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        keys: Vec<smithay::input::keyboard::KeysymHandle<'_>>,
        serial: smithay::utils::Serial,
    ) {
        match self {
            WindowElement::Wayland(w) => KeyboardTarget::enter(w, seat, data, keys, serial),
            WindowElement::X11(x) => KeyboardTarget::enter(x, seat, data, keys, serial),
        }
    }

    fn leave(&self, seat: &smithay::input::Seat<D>, data: &mut D, serial: smithay::utils::Serial) {
        match self {
            WindowElement::Wayland(w) => KeyboardTarget::leave(w, seat, data, serial),
            WindowElement::X11(x) => KeyboardTarget::leave(x, seat, data, serial),
        }
    }

    fn key(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        key: smithay::input::keyboard::KeysymHandle<'_>,
        state: smithay::backend::input::KeyState,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        match self {
            WindowElement::Wayland(w) => {
                KeyboardTarget::key(w, seat, data, key, state, serial, time)
            }
            WindowElement::X11(x) => KeyboardTarget::key(x, seat, data, key, state, serial, time),
        }
    }

    fn modifiers(
        &self,
        seat: &smithay::input::Seat<D>,
        data: &mut D,
        modifiers: smithay::input::keyboard::ModifiersState,
        serial: smithay::utils::Serial,
    ) {
        match self {
            WindowElement::Wayland(w) => {
                KeyboardTarget::modifiers(w, seat, data, modifiers, serial)
            }
            WindowElement::X11(x) => KeyboardTarget::modifiers(x, seat, data, modifiers, serial),
        }
    }
}

impl SpaceElement for WindowElement {
    fn bbox(&self) -> Rectangle<i32, Logical> {
        match self {
            WindowElement::Wayland(w) => w.bbox(),
            WindowElement::X11(x) => x.bbox(),
        }
    }

    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        match self {
            WindowElement::Wayland(w) => w.is_in_input_region(point),
            WindowElement::X11(x) => x.is_in_input_region(point),
        }
    }

    fn set_activate(&self, activated: bool) {
        match self {
            WindowElement::Wayland(w) => w.set_activate(activated),
            WindowElement::X11(x) => x.set_activate(activated),
        }
    }

    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        match self {
            WindowElement::Wayland(w) => w.output_enter(output, overlap),
            WindowElement::X11(x) => x.output_enter(output, overlap),
        }
    }

    fn output_leave(&self, output: &Output) {
        match self {
            WindowElement::Wayland(w) => w.output_leave(output),
            WindowElement::X11(x) => x.output_leave(output),
        }
    }
}

impl WaylandFocus for WindowElement {
    fn wl_surface(
        &self,
    ) -> Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface> {
        match self {
            WindowElement::Wayland(w) => WaylandFocus::wl_surface(w),
            WindowElement::X11(x) => WaylandFocus::wl_surface(x),
        }
    }
}

impl<R> AsRenderElements<R> for WindowElement
where
    R: Renderer + ImportAll,
    <R as Renderer>::TextureId: 'static,
{
    type RenderElement = WaylandSurfaceRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, smithay::utils::Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        match self {
            WindowElement::Wayland(w) => w.render_elements(renderer, location, scale, alpha),
            WindowElement::X11(x) => x.render_elements(renderer, location, scale, alpha),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MagmaWindow {
    pub window: WindowElement,
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

#[derive(Debug, Clone)]
pub struct Workspace {
    windows: Vec<Rc<RefCell<MagmaWindow>>>,
    outputs: Vec<Output>,
    pub layout_tree: BinaryTree,
}

impl Workspace {
    pub fn new() -> Self {
        Workspace {
            windows: Vec::new(),
            outputs: Vec::new(),
            layout_tree: BinaryTree::new(),
        }
    }

    pub fn windows(&self) -> impl Iterator<Item = Ref<'_, WindowElement>> {
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
        self.windows.push(window.clone());
        self.layout_tree
            .insert(window, self.layout_tree.next_split(), 0.5);
        bsp_update_layout(self);
    }

    pub fn remove_window(&mut self, window: &WindowElement) -> Option<Rc<RefCell<MagmaWindow>>> {
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
    ) -> Option<(Ref<'_, WindowElement>, Point<i32, Logical>)> {
        let point = point.into();
        self.windows
            .iter()
            .filter(|e| e.borrow().bbox().to_f64().contains(point))
            .find_map(|e| {
                // we need to offset the point to the location where the surface is actually drawn
                let render_location = e.borrow().render_location();
                if e.borrow()
                    .window
                    .is_in_input_region(&(point - render_location.to_f64()))
                {
                    Some((Ref::map(e.borrow(), |hw| &hw.window), render_location))
                } else {
                    None
                }
            })
    }

    pub fn contains_window(&self, window: &WindowElement) -> bool {
        self.windows.iter().any(|w| &w.borrow().window == window)
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Workspaces {
    workspaces: Vec<Workspace>,
    pub current: u8,
}

impl Workspaces {
    pub fn new(workspaceamount: u8) -> Self {
        Workspaces {
            workspaces: (0..workspaceamount).map(|_| Workspace::new()).collect(),
            current: 0,
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

    pub fn all_windows(&self) -> impl Iterator<Item = Ref<'_, WindowElement>> {
        self.workspaces.iter().flat_map(|w| w.windows())
    }

    pub fn workspace_from_window(&mut self, window: &WindowElement) -> Option<&mut Workspace> {
        self.workspaces
            .iter_mut()
            .find(|w| w.contains_window(window))
    }

    pub fn activate(&mut self, id: u8) {
        self.current = id;
    }
    pub fn move_window_to_workspace(&mut self, window: &WindowElement, workspace: u8) {
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
