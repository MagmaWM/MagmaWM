use smithay::{
    delegate_xdg_decoration, delegate_xdg_shell,
    desktop::{
        PopupKind, PopupManager, WindowSurfaceType, {layer_map_for_output, Window},
    },
    reexports::{
        wayland_protocols::xdg::{
            decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
            shell::server::xdg_toplevel::State as ToplevelState,
        },
        wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
    },
    utils::Serial,
    wayland::{
        compositor::with_states,
        shell::{
            wlr_layer::LayerSurfaceData,
            xdg::{
                decoration::XdgDecorationHandler, PopupSurface, PositionerState, ToplevelSurface,
                XdgPopupSurfaceData, XdgShellHandler, XdgShellState,
                XdgToplevelSurfaceRoleAttributes,
            },
        },
    },
};
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use tracing::warn;

use crate::{
    state::{Backend, MagmaState},
    utils::{
        focus::FocusTarget,
        workspace::{MagmaWindow, Workspaces},
    },
};

impl<BackendData: Backend> XdgShellHandler for MagmaState<BackendData> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface);
        self.workspaces
            .current_mut()
            .add_window(Rc::new(RefCell::new(MagmaWindow {
                window: window.clone(),
                rec: window.geometry(),
            })));
        self.set_input_focus(FocusTarget::Window(window));
    }
    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let window = self
            .workspaces
            .all_windows()
            .find(|w| match w.toplevel() {
                Some(tl) => tl == &surface,
                None => false,
            })
            .unwrap()
            .clone();

        self.workspaces
            .workspace_from_window(&window)
            .unwrap()
            .remove_window(&window);
        self.set_input_focus_auto();
    }
    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
        });
        if let Err(err) = self.popup_manager.track_popup(PopupKind::from(surface)) {
            warn!("Failed to track popup: {}", err);
        }
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        // TODO popup grabs
    }

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
        todo!()
    }
}

delegate_xdg_shell!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

// Should be called on `WlSurface::commit`
pub fn handle_commit(workspaces: &Workspaces, surface: &WlSurface, popup_manager: &PopupManager) {
    if let Some(window) = workspaces.all_windows().find(|w| match w.toplevel() {
        Some(tl) => tl.wl_surface() == surface,
        #[cfg(feature = "xwayland")]
        None => match w.x11_surface() {
            Some(xs) => match xs.wl_surface() {
                Some(s) => &s == surface,
                None => false,
            },
            None => false,
        },
        #[cfg(not(feature = "xwayland"))]
        None => false,
    }) {
        let initial_configure_sent = with_states(surface, |states| {
            match states
                .data_map
                .get::<Mutex<XdgToplevelSurfaceRoleAttributes>>()
            {
                Some(attrs) => attrs.lock().unwrap().initial_configure_sent,
                None => false,
            }
        });
        if !initial_configure_sent {
            let toplevel = window.toplevel();
            if let Some(tl) = toplevel {
                tl.with_pending_state(|state| {
                    state.states.set(ToplevelState::TiledLeft);
                    state.states.set(ToplevelState::TiledRight);
                    state.states.set(ToplevelState::TiledTop);
                    state.states.set(ToplevelState::TiledBottom);
                });
                toplevel.unwrap().send_configure();
            }
        }
    }

    if let Some(output) = workspaces.current().outputs().find(|o| {
        let map = layer_map_for_output(o);
        map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
            .is_some()
    }) {
        let initial_configure_sent = with_states(surface, |states| {
            states
                .data_map
                .get::<LayerSurfaceData>()
                .unwrap()
                .lock()
                .unwrap()
                .initial_configure_sent
        });
        let mut map = layer_map_for_output(output);

        // arrange the layers before sending the initial configure
        // to respect any size the client may have sent
        map.arrange();
        // send the initial configure if relevant
        if !initial_configure_sent {
            let layer = map
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .unwrap();

            layer.layer_surface().send_configure();
        }
    };

    if let Some(popup) = popup_manager.find_popup(surface) {
        let popup = match popup {
            PopupKind::Xdg(ref popup) => popup,
            // Doesn't require configure
            PopupKind::InputMethod(ref _input_popup) => {
                return;
            }
        };
        let initial_configure_sent = with_states(surface, |states| {
            states
                .data_map
                .get::<XdgPopupSurfaceData>()
                .unwrap()
                .lock()
                .unwrap()
                .initial_configure_sent
        });
        if !initial_configure_sent {
            // NOTE: This should never fail as the initial configure is always
            // allowed.
            popup.send_configure().expect("initial configure failed");
        }
    };
}

// Disable decorations
impl<BackendData: Backend> XdgDecorationHandler for MagmaState<BackendData> {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            // Advertise server side decoration
            state.decoration_mode = Some(Mode::ServerSide);
        });
        toplevel.send_configure();
    }

    fn request_mode(
        &mut self,
        _toplevel: ToplevelSurface,
        _mode: smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
    ) {
    }

    fn unset_mode(&mut self, _toplevel: ToplevelSurface) {}
}

delegate_xdg_decoration!(@<BackendData: Backend + 'static> MagmaState<BackendData>);
