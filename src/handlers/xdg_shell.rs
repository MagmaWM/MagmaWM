use std::{sync::Mutex, rc::Rc, cell::RefCell};

use smithay::{wayland::{shell::xdg::{XdgShellHandler, XdgShellState, ToplevelSurface, PopupSurface, PositionerState, XdgToplevelSurfaceRoleAttributes}, compositor::with_states}, desktop::{Window}, reexports::{wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface}}, utils::Serial, delegate_xdg_shell};

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
            window.toplevel().send_configure();
        }
    }
}
