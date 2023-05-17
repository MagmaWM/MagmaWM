use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_data_device, delegate_layer_shell, delegate_output,
    delegate_primary_selection, delegate_seat, delegate_shm,
    desktop::{layer_map_for_output, LayerSurface},
    input::{SeatHandler, SeatState},
    output::Output,
    reexports::wayland_server::{
        protocol::{wl_output::WlOutput, wl_surface::WlSurface},
        Resource,
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{get_parent, is_sync_subsurface, CompositorHandler, CompositorState},
        data_device::{
            set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, ServerDndGrabHandler,
        },
        primary_selection::{set_primary_focus, PrimarySelectionHandler},
        seat::WaylandFocus,
        shell::wlr_layer::{
            Layer, LayerSurface as WlrLayerSurface, WlrLayerShellHandler, WlrLayerShellState,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::{
    state::{Backend, MagmaState},
    utils::{focus::FocusTarget, tiling::bsp_update_layout}, ipc::generated::magma_ipc,
};

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
    type KeyboardFocus = FocusTarget;
    type PointerFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<MagmaState<BackendData>> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &smithay::input::Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
    fn focus_changed(&mut self, seat: &smithay::input::Seat<Self>, focused: Option<&FocusTarget>) {
        let dh = &self.dh;

        let focus = focused
            .and_then(WaylandFocus::wl_surface)
            .and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, focus.clone());
        set_primary_focus(dh, seat, focus);

        if let Some(focus_target) = focused {
            match focus_target {
                FocusTarget::Window(w) => {
                    for window in self.workspaces.all_windows() {
                        if window.eq(w) {
                            window.set_activated(true);
                        } else {
                            window.set_activated(false);
                        }
                        window.toplevel().send_configure();
                    }
                }
                FocusTarget::LayerSurface(_) => {
                    for window in self.workspaces.all_windows() {
                        window.set_activated(false);
                        window.toplevel().send_configure();
                    }
                }
                FocusTarget::Popup(_) => {}
            };
        }
    }
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

impl<BackendData: Backend> PrimarySelectionHandler for MagmaState<BackendData> {
    fn primary_selection_state(
        &self,
    ) -> &smithay::wayland::primary_selection::PrimarySelectionState {
        &self.primary_selection_state
    }
}

delegate_primary_selection!(@<BackendData: Backend + 'static> MagmaState<BackendData>);
//
// Wl Output & Xdg Output
//

delegate_output!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

impl<BackendData: Backend> WlrLayerShellHandler for MagmaState<BackendData> {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: WlrLayerSurface,
        output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .unwrap_or_else(|| self.workspaces.current().outputs().next().unwrap().clone());
        let mut map = layer_map_for_output(&output);
        let layer_surface = LayerSurface::new(surface, namespace);
        map.map_layer(&layer_surface).unwrap();
        self.set_input_focus(FocusTarget::LayerSurface(layer_surface));
        drop(map);
        for workspace in self.workspaces.iter() {
            bsp_update_layout(workspace);
        }
    }

    fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
        if let Some((mut map, layer)) = self.workspaces.outputs().find_map(|o| {
            let map = layer_map_for_output(o);
            let layer = map
                .layers()
                .find(|&layer| layer.layer_surface() == &surface)
                .cloned();
            layer.map(|layer| (map, layer))
        }) {
            map.unmap_layer(&layer);
        }
        self.set_input_focus_auto();
        for workspace in self.workspaces.iter() {
            bsp_update_layout(workspace);
        }
    }
}

delegate_layer_shell!(@<BackendData: Backend + 'static> MagmaState<BackendData>);
