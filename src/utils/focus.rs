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
use tracing::debug;

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
            FocusTarget::Window(w) => match w.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    PointerTarget::enter(w.wl_surface(), seat, data, event)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(w) => PointerTarget::enter(&w, seat, data, event),
                    None => debug!("Pointer entered non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(w) => match w.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    PointerTarget::motion(w.wl_surface(), seat, data, event)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(w) => PointerTarget::motion(&w, seat, data, event),
                    None => debug!("Pointer motion on non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(w) => match w.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    PointerTarget::relative_motion(w.wl_surface(), seat, data, event)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(w) => PointerTarget::relative_motion(&w, seat, data, event),
                    None => debug!("Relative pointer movement on non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(w) => match w.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    PointerTarget::button(w.wl_surface(), seat, data, event)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(w) => PointerTarget::button(&w, seat, data, event),
                    None => debug!("Button press on non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(w) => match w.underlying_surface() {
                WindowSurface::Wayland(w) => PointerTarget::axis(w.wl_surface(), seat, data, frame),
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(w) => PointerTarget::axis(&w, seat, data, frame),
                    None => debug!("Scroll on non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(w) => match w.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    PointerTarget::leave(w.wl_surface(), seat, data, serial, time)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(w) => PointerTarget::leave(&w, seat, data, serial, time),
                    None => debug!("Attempted un-focus on non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(win) => match win.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    KeyboardTarget::enter(w.wl_surface(), seat, data, keys, serial)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(s) => KeyboardTarget::enter(&s, seat, data, keys, serial),
                    None => debug!("Attempted to keyboard focus non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(win) => match win.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    KeyboardTarget::leave(w.wl_surface(), seat, data, serial)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(s) => KeyboardTarget::leave(&s, seat, data, serial),
                    None => debug!("Attempted to keyboard un-focus non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(win) => match win.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    KeyboardTarget::key(w.wl_surface(), seat, data, key, state, serial, time)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(s) => KeyboardTarget::key(&s, seat, data, key, state, serial, time),
                    None => debug!("Ignored keypress on non-visible xwayland window"),
                },
            },
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
            FocusTarget::Window(win) => match win.underlying_surface() {
                WindowSurface::Wayland(w) => {
                    KeyboardTarget::modifiers(w.wl_surface(), seat, data, modifiers, serial)
                }
                #[cfg(feature = "xwayland")]
                WindowSurface::X11(x) => match x.wl_surface() {
                    Some(s) => KeyboardTarget::modifiers(&s, seat, data, modifiers, serial),
                    None => debug!("Ignored modifier keypress on non-visible xwayland window"),
                },
            },
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
