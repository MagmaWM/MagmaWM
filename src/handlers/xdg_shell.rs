use std::{sync::Mutex, rc::Rc, cell::RefCell};

use smithay::{wayland::{shell::xdg::{XdgShellHandler, XdgShellState, ToplevelSurface, PopupSurface, PositionerState, XdgToplevelSurfaceRoleAttributes, decoration::XdgDecorationHandler}, compositor::with_states}, desktop::{Window}, reexports::{wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface}, wayland_protocols::xdg::{decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode, shell::server::xdg_toplevel::State as ToplevelState}}, utils::Serial, delegate_xdg_shell, delegate_xdg_decoration};

use crate::{state::{Backend, MagmaState}, utils::workspace::{MagmaWindow, Workspaces}};


impl<BackendData: Backend> XdgShellHandler for MagmaState<BackendData> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new(surface);
        self.workspaces.current_mut().add_window(Rc::new(RefCell::new(MagmaWindow { window: window.clone(), rec: window.geometry() })))
    }
    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let window = self
        .workspaces
        .all_windows()
        .find(|w| w.toplevel() == &surface)
        .unwrap()
        .clone();

    self.workspaces.workspace_from_window(&window).unwrap().remove_window(&window);
    }
    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        //TODO map popups
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        // TODO popup grabs
    }
}

delegate_xdg_shell!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

// Should be called on `WlSurface::commit`
pub fn handle_commit(workspaces: &Workspaces, surface: &WlSurface) {
    if let Some(window) = workspaces
        .all_windows()
        .find(|w| w.toplevel().wl_surface() == surface)
    {
        let initial_configure_sent = with_states(surface, |states| {
            states
                .data_map
                .get::<Mutex<XdgToplevelSurfaceRoleAttributes>>()
                .unwrap()
                .lock()
                .unwrap()
                .initial_configure_sent
        });
        if !initial_configure_sent {
            let toplevel = window.toplevel();
            toplevel.with_pending_state(|state| {
                state.states.set(ToplevelState::TiledLeft);
                state.states.set(ToplevelState::TiledRight);
                state.states.set(ToplevelState::TiledTop);
                state.states.set(ToplevelState::TiledBottom);
            });
            toplevel.send_configure();
        }
    }
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
