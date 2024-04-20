use std::{
    ffi::OsString,
    sync::Arc,
    time::{Duration, Instant},
};

use once_cell::sync::Lazy;
use smithay::{
    desktop::{layer_map_for_output, PopupManager, Window},
    input::{keyboard::XkbConfig, Seat, SeatState},
    reexports::{
        calloop::{
            generic::Generic,
            timer::{TimeoutAction, Timer},
            Interest, LoopHandle, LoopSignal, Mode, PostAction,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            Display, DisplayHandle,
        },
    },
    utils::{Logical, Point, Rectangle},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        cursor_shape::CursorShapeManagerState,
        output::OutputManagerState,
        selection::{data_device::DataDeviceState, primary_selection::PrimarySelectionState},
        shell::{
            wlr_layer::{Layer as WlrLayer, WlrLayerShellState},
            xdg::{decoration::XdgDecorationState, XdgShellState},
        },
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};
use tracing::warn;

use crate::utils::{focus::FocusTarget, workspace::Workspaces, xcursor::Xcursor};
use crate::{
    config::{load_config, Config},
    debug::MagmaDebug,
};

pub struct CalloopData<BackendData: Backend + 'static> {
    pub state: MagmaState<BackendData>,
    pub display_handle: DisplayHandle,
}

pub trait Backend {
    fn seat_name(&self) -> String;
}

pub static CONFIG: Lazy<Config> = Lazy::new(load_config);

pub struct MagmaState<BackendData: Backend + 'static> {
    pub dh: DisplayHandle,
    pub backend_data: BackendData,
    pub start_time: Instant,
    pub loop_handle: LoopHandle<'static, CalloopData<BackendData>>,
    pub loop_signal: LoopSignal,

    // protocol state
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub data_device_state: DataDeviceState,
    pub primary_selection_state: PrimarySelectionState,
    pub seat_state: SeatState<MagmaState<BackendData>>,
    pub layer_shell_state: WlrLayerShellState,
    pub popup_manager: PopupManager,
    pub cursor_shape_manager: CursorShapeManagerState,

    pub seat: Seat<Self>,
    pub seat_name: String,
    pub socket_name: OsString,

    pub workspaces: Workspaces,
    pub pointer_location: Point<f64, Logical>,
    pub xcursor: Xcursor,

    #[cfg(feature = "debug")]
    pub debug: MagmaDebug,
}

impl<BackendData: Backend + 'static> MagmaState<BackendData> {
    pub fn new(
        loop_handle: LoopHandle<'static, CalloopData<BackendData>>,
        loop_signal: LoopSignal,
        display: Display<MagmaState<BackendData>>,
        backend_data: BackendData,
    ) -> Self {
        let start_time = Instant::now();

        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self>(&dh);
        let seat_name = backend_data.seat_name();
        let mut seat = seat_state.new_wl_seat(&dh, seat_name.clone());
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let cursor_shape_manager = CursorShapeManagerState::new::<Self>(&dh);

        let conf = CONFIG.xkb.clone();
        if let Err(err) = seat.add_keyboard((&conf).into(), 200, 25) {
            warn!(
                ?err,
                "Failed to load provided xkb config. Trying default...",
            );
            seat.add_keyboard(XkbConfig::default(), 200, 25)
                .expect("Failed to load xkb configuration files");
        }
        seat.add_pointer();
        let xcursor = Xcursor::new();

        let workspaces = Workspaces::new(CONFIG.workspaces);

        // Creates a new listening socket, automatically choosing the next available `wayland` socket name.
        let listening_socket = ListeningSocketSource::new_auto().unwrap();

        // Get the name of the listening socket.
        // Clients will connect to this socket.
        let socket_name = listening_socket.socket_name().to_os_string();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                // Inside the callback, you should insert the client into the display.
                //
                // You may also associate some data with the client when inserting the client.
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        // You also need to add the display itself to the event loop, so that client events will be processed by wayland-server.
        loop_handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    unsafe {
                        display
                            .get_mut()
                            .dispatch_clients(&mut state.state)
                            .unwrap()
                    };
                    Ok(PostAction::Continue)
                },
            )
            .expect("Failed to init wayland server source");

        loop_handle
            .insert_source(
                Timer::from_duration(Duration::from_millis(xcursor.get_curr_image().delay.into())),
                |_, _, d| {
                    d.state.xcursor.tick();
                    TimeoutAction::ToDuration(Duration::from_millis(
                        d.state.xcursor.get_curr_image().delay.into(),
                    ))
                },
            )
            .expect("Failed to init cursor tick event source");

        Self {
            loop_handle,
            dh,
            backend_data,
            start_time,
            seat_name,
            socket_name,
            compositor_state,
            xdg_shell_state,
            xdg_decoration_state,
            loop_signal,
            shm_state,
            output_manager_state,
            popup_manager: PopupManager::default(),
            seat_state,
            data_device_state,
            primary_selection_state,
            layer_shell_state,
            cursor_shape_manager,
            seat,
            workspaces,
            pointer_location: Point::from((0.0, 0.0)),
            xcursor,
            #[cfg(feature = "debug")]
            debug: MagmaDebug {
                egui: smithay_egui::EguiState::new(Rectangle::from_loc_and_size(
                    (0, 0),
                    (800, 600),
                )),
                active: false,
                fps: Default::default(),
            },
        }
    }

    pub fn window_under(&mut self) -> Option<(Window, Point<i32, Logical>)> {
        let pos = self.pointer_location;
        self.workspaces
            .current()
            .window_under(pos)
            .map(|(w, p)| (w.clone(), p))
    }
    pub fn surface_under(&self) -> Option<(FocusTarget, Point<i32, Logical>)> {
        let pos = self.pointer_location;
        let output = self.workspaces.current().outputs().find(|o| {
            let geometry = self.workspaces.current().output_geometry(o).unwrap();
            geometry.contains(pos.to_i32_round())
        })?;
        let output_geo = self.workspaces.current().output_geometry(output).unwrap();
        let layers = layer_map_for_output(output);

        let mut under = None;
        if let Some(layer) = layers
            .layer_under(WlrLayer::Overlay, pos)
            .or_else(|| layers.layer_under(WlrLayer::Top, pos))
        {
            let layer_loc = layers.layer_geometry(layer).unwrap().loc;
            under = Some((layer.clone().into(), output_geo.loc + layer_loc))
        } else if let Some((window, location)) = self.workspaces.current().window_under(pos) {
            under = Some((window.clone().into(), location));
        } else if let Some(layer) = layers
            .layer_under(WlrLayer::Bottom, pos)
            .or_else(|| layers.layer_under(WlrLayer::Background, pos))
        {
            let layer_loc = layers.layer_geometry(layer).unwrap().loc;
            under = Some((layer.clone().into(), output_geo.loc + layer_loc));
        };
        under
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
