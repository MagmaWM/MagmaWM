use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    desktop::Window,
    input::{SeatHandler, SeatState},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        buffer::BufferHandler,
        compositor::{get_parent, is_sync_subsurface, CompositorHandler, CompositorState},
        data_device::{ClientDndGrabHandler, DataDeviceHandler, ServerDndGrabHandler},
        shm::{ShmHandler, ShmState},
    },
};

use crate::state::{Backend, MagmaState};

pub mod input;
pub mod xdg_shell;

impl<BackendData: Backend> CompositorHandler for MagmaState<BackendData> {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .workspaces
                .all_windows()
                .find(|w| w.toplevel().wl_surface() == &root)
            {
                window.on_commit();
            }
        };
        self.popup_manager.commit(surface);
        xdg_shell::handle_commit(&self.workspaces, surface, &self.popup_manager);
    }
}

delegate_compositor!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

impl<BackendData: Backend> BufferHandler for MagmaState<BackendData> {
    fn buffer_destroyed(
        &mut self,
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
    }
}

impl<BackendData: Backend> ShmHandler for MagmaState<BackendData> {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

delegate_shm!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

impl<BackendData: Backend> SeatHandler for MagmaState<BackendData> {
    type KeyboardFocus = Window;
    type PointerFocus = Window;

    fn seat_state(&mut self) -> &mut SeatState<MagmaState<BackendData>> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &smithay::input::Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
    fn focus_changed(&mut self, _seat: &smithay::input::Seat<Self>, _focused: Option<&Window>) {}
}

delegate_seat!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

//
// Wl Data Device
//

impl<BackendData: Backend> DataDeviceHandler for MagmaState<BackendData> {
    fn data_device_state(&self) -> &smithay::wayland::data_device::DataDeviceState {
        &self.data_device_state
    }
}

impl<BackendData: Backend> ClientDndGrabHandler for MagmaState<BackendData> {}
impl<BackendData: Backend> ServerDndGrabHandler for MagmaState<BackendData> {}

delegate_data_device!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

//
// Wl Output & Xdg Output
//

delegate_output!(@<BackendData: Backend + 'static> MagmaState<BackendData>);
