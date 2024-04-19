use smithay::desktop::{Window, WindowSurface};
pub use smithay::{
    backend::input::KeyState,
    desktop::{LayerSurface, PopupKind},
    input::{
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent, PointerTarget, RelativeMotionEvent},
        Seat,
    },
    reexports::wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface, Resource},
    utils::{IsAlive, Serial},
    wayland::seat::WaylandFocus,
};

use crate::state::{Backend, MagmaState};

#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Window(Window),
    LayerSurface(LayerSurface),
    Popup(PopupKind),
}

impl IsAlive for FocusTarget {
    fn alive(&self) -> bool {
        match self {
            FocusTarget::Window(w) => w.alive(),
            FocusTarget::LayerSurface(l) => l.alive(),
            FocusTarget::Popup(p) => p.alive(),
        }
    }
}

impl From<FocusTarget> for WlSurface {
    fn from(target: FocusTarget) -> Self {
        target.wl_surface().unwrap()
    }
}

impl<BackendData: Backend> PointerTarget<MagmaState<BackendData>> for FocusTarget {
    fn enter(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        event: &MotionEvent,
    ) {
        match self {
            FocusTarget::Window(w) => {
                PointerTarget::enter(&w.wl_surface().unwrap(), seat, data, event)
            }
            FocusTarget::LayerSurface(l) => PointerTarget::enter(l.wl_surface(), seat, data, event),
            FocusTarget::Popup(p) => PointerTarget::enter(p.wl_surface(), seat, data, event),
        }
    }
    fn motion(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        event: &MotionEvent,
    ) {
        match self {
            FocusTarget::Window(w) => {
                PointerTarget::motion(&w.wl_surface().unwrap(), seat, data, event)
            }
            FocusTarget::LayerSurface(l) => {
                PointerTarget::motion(l.wl_surface(), seat, data, event)
            }
            FocusTarget::Popup(p) => PointerTarget::motion(p.wl_surface(), seat, data, event),
        }
    }
    fn relative_motion(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        event: &RelativeMotionEvent,
    ) {
        match self {
            FocusTarget::Window(w) => {
                PointerTarget::relative_motion(&w.wl_surface().unwrap(), seat, data, event)
            }
            FocusTarget::LayerSurface(l) => {
                PointerTarget::relative_motion(l.wl_surface(), seat, data, event)
            }
            FocusTarget::Popup(p) => {
                PointerTarget::relative_motion(p.wl_surface(), seat, data, event)
            }
        }
    }
    fn button(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        event: &ButtonEvent,
    ) {
        match self {
            FocusTarget::Window(w) => {
                PointerTarget::button(&w.wl_surface().unwrap(), seat, data, event)
            }
            FocusTarget::LayerSurface(l) => {
                PointerTarget::button(l.wl_surface(), seat, data, event)
            }
            FocusTarget::Popup(p) => PointerTarget::button(p.wl_surface(), seat, data, event),
        }
    }
    fn axis(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        frame: AxisFrame,
    ) {
        match self {
            FocusTarget::Window(w) => {
                PointerTarget::axis(&w.wl_surface().unwrap(), seat, data, frame)
            }
            FocusTarget::LayerSurface(l) => PointerTarget::axis(l.wl_surface(), seat, data, frame),
            FocusTarget::Popup(p) => PointerTarget::axis(p.wl_surface(), seat, data, frame),
        }
    }
    fn leave(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        serial: Serial,
        time: u32,
    ) {
        match self {
            FocusTarget::Window(w) => {
                PointerTarget::leave(&w.wl_surface().unwrap(), seat, data, serial, time)
            }
            FocusTarget::LayerSurface(l) => {
                PointerTarget::leave(l.wl_surface(), seat, data, serial, time)
            }
            FocusTarget::Popup(p) => PointerTarget::leave(p.wl_surface(), seat, data, serial, time),
        }
    }

    fn frame(&self, _seat: &Seat<MagmaState<BackendData>>, _data: &mut MagmaState<BackendData>) {
        todo!()
    }

    fn gesture_swipe_begin(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GestureSwipeBeginEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_update(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GestureSwipeUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_swipe_end(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GestureSwipeEndEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_begin(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GesturePinchBeginEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_update(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GesturePinchUpdateEvent,
    ) {
        todo!()
    }

    fn gesture_pinch_end(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GesturePinchEndEvent,
    ) {
        todo!()
    }

    fn gesture_hold_begin(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GestureHoldBeginEvent,
    ) {
        todo!()
    }

    fn gesture_hold_end(
        &self,
        _seat: &Seat<MagmaState<BackendData>>,
        _data: &mut MagmaState<BackendData>,
        _event: &smithay::input::pointer::GestureHoldEndEvent,
    ) {
        todo!()
    }
}

impl<BackendData: Backend> KeyboardTarget<MagmaState<BackendData>> for FocusTarget {
    fn enter(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            FocusTarget::Window(w) => {
                let WindowSurface::Wayland(w) = w.underlying_surface();
                KeyboardTarget::enter(w.wl_surface(), seat, data, keys, serial)
            }
            FocusTarget::LayerSurface(l) => {
                KeyboardTarget::enter(l.wl_surface(), seat, data, keys, serial)
            }
            FocusTarget::Popup(p) => {
                KeyboardTarget::enter(p.wl_surface(), seat, data, keys, serial)
            }
        }
    }
    fn leave(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        serial: Serial,
    ) {
        match self {
            FocusTarget::Window(w) => {
                let WindowSurface::Wayland(w) = w.underlying_surface();
                KeyboardTarget::leave(w.wl_surface(), seat, data, serial)
            }
            FocusTarget::LayerSurface(l) => {
                KeyboardTarget::leave(l.wl_surface(), seat, data, serial)
            }
            FocusTarget::Popup(p) => KeyboardTarget::leave(p.wl_surface(), seat, data, serial),
        }
    }
    fn key(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        match self {
            FocusTarget::Window(w) => {
                let WindowSurface::Wayland(w) = w.underlying_surface();
                KeyboardTarget::key(w.wl_surface(), seat, data, key, state, serial, time)
            }
            FocusTarget::LayerSurface(l) => {
                KeyboardTarget::key(l.wl_surface(), seat, data, key, state, serial, time)
            }
            FocusTarget::Popup(p) => {
                KeyboardTarget::key(p.wl_surface(), seat, data, key, state, serial, time)
            }
        }
    }
    fn modifiers(
        &self,
        seat: &Seat<MagmaState<BackendData>>,
        data: &mut MagmaState<BackendData>,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        match self {
            FocusTarget::Window(w) => {
                let WindowSurface::Wayland(w) = w.underlying_surface();
                KeyboardTarget::modifiers(w.wl_surface(), seat, data, modifiers, serial)
            }
            FocusTarget::LayerSurface(l) => {
                KeyboardTarget::modifiers(l.wl_surface(), seat, data, modifiers, serial)
            }
            FocusTarget::Popup(p) => {
                KeyboardTarget::modifiers(p.wl_surface(), seat, data, modifiers, serial)
            }
        }
    }
}

impl WaylandFocus for FocusTarget {
    fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            FocusTarget::Window(w) => w.wl_surface(),
            FocusTarget::LayerSurface(l) => Some(l.wl_surface().clone()),
            FocusTarget::Popup(p) => Some(p.wl_surface().clone()),
        }
    }
    fn same_client_as(&self, object_id: &ObjectId) -> bool {
        match self {
            FocusTarget::Window(w) => w.same_client_as(object_id),
            FocusTarget::LayerSurface(l) => l.wl_surface().id().same_client_as(object_id),
            FocusTarget::Popup(p) => p.wl_surface().id().same_client_as(object_id),
        }
    }
}

impl From<Window> for FocusTarget {
    fn from(w: Window) -> Self {
        FocusTarget::Window(w)
    }
}

impl From<LayerSurface> for FocusTarget {
    fn from(l: LayerSurface) -> Self {
        FocusTarget::LayerSurface(l)
    }
}

impl From<PopupKind> for FocusTarget {
    fn from(p: PopupKind) -> Self {
        FocusTarget::Popup(p)
    }
}
