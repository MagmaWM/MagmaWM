use std::{collections::HashMap, os::fd::FromRawFd, path::PathBuf, time::Duration};

use smithay::{
    backend::{
        allocator::{
            gbm::{self, GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            self, compositor::DrmCompositor, DrmDevice, DrmDeviceFd, DrmError, DrmNode, NodeType,
        },
        egl::{EGLDevice, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            element::{
                surface::WaylandSurfaceRenderElement,
                texture::{TextureBuffer, TextureRenderElement},
                AsRenderElements,
            },
            gles::{GlesRenderer, GlesTexture},
            multigpu::{gbm::GbmGlesBackend, GpuManager, MultiRenderer, MultiTexture},
        },
        session::{libseat::LibSeatSession, Session},
        udev::{self, UdevBackend, UdevEvent},
        SwapBuffersError,
    },
    desktop::{layer_map_for_output, space::SpaceElement, LayerSurface},
    output::{Mode as WlMode, Output, PhysicalProperties},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop, RegistrationToken,
        },
        drm::{
            control::{crtc, ModeTypeFlags},
            Device as DrmDeviceTrait, SystemError,
        },
        input::Libinput,
        nix::fcntl::OFlag,
        wayland_server::{backend::GlobalId, Display},
    },
    utils::{DeviceFd, Scale, Size, Transform},
    wayland::shell::wlr_layer::Layer,
};
use smithay_drm_extras::{
    drm_scanner::{DrmScanEvent, DrmScanner},
    edid::EdidInfo,
};
use tracing::{error, info, trace, warn};

use crate::{
    state::{Backend, CalloopData, MagmaState, CONFIG},
    utils::render::CustomRenderElements,
};

static CURSOR_DATA: &[u8] = include_bytes!("../../resources/cursor.rgba");

const SUPPORTED_FORMATS: &[Fourcc] = &[
    Fourcc::Abgr2101010,
    Fourcc::Argb2101010,
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];

pub type GbmDrmCompositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmDevice<DrmDeviceFd>, (), DrmDeviceFd>;

pub struct UdevData {
    pub session: LibSeatSession,
    _primary_gpu: DrmNode,
    gpus: GpuManager<GbmGlesBackend<GlesRenderer>>,
    devices: HashMap<DrmNode, Device>,
}

impl Backend for UdevData {
    fn seat_name(&self) -> String {
        self.session.seat()
    }
}
pub struct Device {
    pub surfaces: HashMap<crtc::Handle, Surface>,
    pub gbm: GbmDevice<DrmDeviceFd>,
    pub drm: DrmDevice,
    pub drm_scanner: DrmScanner,
    pub render_node: DrmNode,
    pub registration_token: RegistrationToken,
}

pub struct Surface {
    _device_id: DrmNode,
    _render_node: DrmNode,
    global: GlobalId,
    compositor: GbmDrmCompositor,
    _output: Output,
    pointer_texture: TextureBuffer<MultiTexture>,
}

pub fn init_udev() {
    let mut event_loop: EventLoop<CalloopData<UdevData>> = EventLoop::try_new().unwrap();
    let mut display: Display<MagmaState<UdevData>> = Display::new().unwrap();

    /*
     * Initialize session
     */
    let (session, notifier) = match LibSeatSession::new() {
        Ok(ret) => ret,
        Err(err) => {
            error!("Could not initialize a session: {}", err);
            return;
        }
    };

    /*
     * Initialize the compositor
     */
    let (primary_gpu, _) = primary_gpu(&session.seat());
    info!("Using {} as primary gpu.", primary_gpu);

    let gpus = GpuManager::new(Default::default()).unwrap();

    let data = UdevData {
        session,
        _primary_gpu: primary_gpu,
        gpus,
        devices: HashMap::new(),
    };

    let mut state = MagmaState::new(
        event_loop.handle(),
        event_loop.get_signal(),
        &mut display,
        data,
    );

    /*
     * Add input source
     */
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        state.backend_data.session.clone().into(),
    );
    libinput_context
        .udev_assign_seat(&state.backend_data.session.seat())
        .unwrap();

    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    event_loop
        .handle()
        .insert_source(libinput_backend, move |event, _, calloopdata| {
            calloopdata.state.process_input_event(event);
        })
        .unwrap();

    event_loop
        .handle()
        .insert_source(notifier, move |_, _, _| {
            //TODO
        })
        .unwrap();

    /*
     * Initialize Udev
     */

    let backend = UdevBackend::new(&state.seat_name).unwrap();
    for (device_id, path) in backend.device_list() {
        state.on_udev_event(
            UdevEvent::Added {
                device_id,
                path: path.to_owned(),
            },
            &mut display,
        );
    }

    event_loop
        .handle()
        .insert_source(backend, |event, _, calloopdata| {
            calloopdata
                .state
                .on_udev_event(event, &mut calloopdata.display)
        })
        .unwrap();

    let mut calloopdata = CalloopData { state, display };

    std::env::set_var("WAYLAND_DISPLAY", &calloopdata.state.socket_name);

    for command in &CONFIG.autostart {
        if let Err(err) = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .spawn()
        {
            info!("{} {} {}", err, "Failed to spawn \"{}\"", command);
        }
    }

    event_loop
        .run(None, &mut calloopdata, move |data| {
            data.state
                .workspaces
                .all_windows()
                .for_each(|e| e.refresh());
            data.display.flush_clients().unwrap();
        })
        .unwrap();
}

pub fn primary_gpu(seat: &str) -> (DrmNode, PathBuf) {
    // TODO: can't this be in smithay?
    // primary_gpu() does the same thing anyway just without `NodeType::Render` check
    // so perhaps `primary_gpu(seat, node_type)`?
    udev::primary_gpu(seat)
        .unwrap()
        .and_then(|p| {
            DrmNode::from_path(&p)
                .ok()?
                .node_with_type(NodeType::Render)?
                .ok()
                .map(|node| (node, p))
        })
        .unwrap_or_else(|| {
            udev::all_gpus(seat)
                .unwrap()
                .into_iter()
                .find_map(|p| {
                    DrmNode::from_path(&p)
                        .ok()?
                        .node_with_type(NodeType::Render)?
                        .ok()
                        .map(|node| (node, p))
                })
                .expect("No GPU!")
        })
}

// Udev
impl MagmaState<UdevData> {
    pub fn on_udev_event(&mut self, event: UdevEvent, display: &mut Display<MagmaState<UdevData>>) {
        match event {
            UdevEvent::Added { device_id, path } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    self.on_device_added(node, path, display);
                }
            }
            UdevEvent::Changed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    self.on_device_changed(node, display);
                }
            }
            UdevEvent::Removed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    self.on_device_removed(node);
                }
            }
        }
    }

    fn on_device_added(
        &mut self,
        node: DrmNode,
        path: PathBuf,
        display: &mut Display<MagmaState<UdevData>>,
    ) {
        let fd = self
            .backend_data
            .session
            .open(
                &path,
                OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NOCTTY | OFlag::O_NONBLOCK,
            )
            .unwrap();

        let fd = DrmDeviceFd::new(unsafe { DeviceFd::from_raw_fd(fd) });

        let (drm, drm_notifier) = drm::DrmDevice::new(fd, true).unwrap();

        let gbm = gbm::GbmDevice::new(drm.device_fd().clone()).unwrap();

        // Make sure display is dropped before we call add_node
        let render_node =
            match EGLDevice::device_for_display(&EGLDisplay::new(gbm.clone()).unwrap())
                .ok()
                .and_then(|x| x.try_get_render_node().ok().flatten())
            {
                Some(node) => node,
                None => node,
            };

        self.backend_data
            .gpus
            .as_mut()
            .add_node(render_node, gbm.clone())
            .unwrap();

        let registration_token = self
            .loop_handle
            .insert_source(drm_notifier, move |event, meta, calloopdata| {
                calloopdata.state.on_drm_event(node, event, meta);
            })
            .unwrap();

        self.backend_data.devices.insert(
            node,
            Device {
                drm,
                gbm,
                drm_scanner: Default::default(),
                surfaces: Default::default(),
                render_node,
                registration_token,
            },
        );

        self.on_device_changed(node, display);
    }

    fn on_device_changed(&mut self, node: DrmNode, display: &mut Display<MagmaState<UdevData>>) {
        if let Some(device) = self.backend_data.devices.get_mut(&node) {
            for event in device.drm_scanner.scan_connectors(&device.drm) {
                self.on_connector_event(node, event, display);
            }
        }
    }

    fn on_device_removed(&mut self, node: DrmNode) {
        if let Some(device) = self.backend_data.devices.get_mut(&node) {
            self.backend_data
                .gpus
                .as_mut()
                .remove_node(&device.render_node);

            for surface in device.surfaces.values() {
                self.dh
                    .disable_global::<MagmaState<UdevData>>(surface.global.clone());
            }
        }
    }
}

// Drm
impl MagmaState<UdevData> {
    pub fn on_drm_event(
        &mut self,
        node: DrmNode,
        event: drm::DrmEvent,
        _meta: &mut Option<drm::DrmEventMetadata>,
    ) {
        match event {
            drm::DrmEvent::VBlank(crtc) => {
                let device = self.backend_data.devices.get_mut(&node).unwrap();
                let surface = device.surfaces.get_mut(&crtc).unwrap();
                surface.compositor.frame_submitted().ok();
                self.render(node, crtc).ok();
            }
            drm::DrmEvent::Error(_) => {}
        }
    }

    pub fn on_connector_event(
        &mut self,
        node: DrmNode,
        event: DrmScanEvent,
        display: &mut Display<MagmaState<UdevData>>,
    ) {
        let device = if let Some(device) = self.backend_data.devices.get_mut(&node) {
            device
        } else {
            error!("Received connector event for unknown device: {:?}", node);
            return;
        };

        match event {
            DrmScanEvent::Connected {
                connector,
                crtc: Some(crtc),
            } => {
                let mut renderer = self
                    .backend_data
                    .gpus
                    .single_renderer(&device.render_node)
                    .unwrap();

                let name = format!(
                    "{}-{}",
                    connector.interface().as_str(),
                    connector.interface_id()
                );
                info!("New output connected, name: {}", name);
                let drm_mode = if CONFIG.outputs.contains_key(&name) {
                    let output_config = &CONFIG.outputs[&name];
                    *connector
                        .modes()
                        .iter()
                        .filter(|mode| {
                            let (x, y) = mode.size();
                            Size::from((x as i32, y as i32)) == output_config.mode_size()
                        })
                        // and then select the closest refresh rate (e.g. to match 59.98 as 60)
                        .min_by_key(|mode| {
                            let refresh_rate = WlMode::from(**mode).refresh;
                            (output_config.mode_refresh() as i32 - refresh_rate).abs()
                        })
                        .expect("No matching mode found for output config")
                } else {
                    *connector
                        .modes()
                        .iter()
                        .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
                        .unwrap_or(&connector.modes()[0])
                };

                let drm_surface = device
                    .drm
                    .create_surface(crtc, drm_mode, &[connector.handle()])
                    .unwrap();

                let (make, model) = EdidInfo::for_connector(&device.drm, connector.handle())
                    .map(|info| (info.manufacturer, info.model))
                    .unwrap_or_else(|| ("Unknown".into(), "Unknown".into()));

                let (w, h) = connector.size().unwrap_or((0, 0));
                let output = Output::new(
                    name,
                    PhysicalProperties {
                        size: (w as i32, h as i32).into(),
                        subpixel: smithay::output::Subpixel::Unknown,
                        make,
                        model,
                    },
                );
                let global = output.create_global::<MagmaState<UdevData>>(&display.handle());
                let output_mode = WlMode::from(drm_mode);
                output.set_preferred(output_mode);
                output.change_current_state(
                    Some(output_mode),
                    Some(Transform::Normal),
                    Some(smithay::output::Scale::Integer(1)),
                    None,
                );
                let render_formats = renderer
                    .as_mut()
                    .egl_context()
                    .dmabuf_render_formats()
                    .clone();
                let gbm_allocator =
                    GbmAllocator::new(device.gbm.clone(), GbmBufferFlags::RENDERING);

                let driver = match device.drm.get_driver() {
                    Ok(driver) => driver,
                    Err(err) => {
                        warn!("Failed to query drm driver: {}", err);
                        return;
                    }
                };

                let mut planes = match drm_surface.planes() {
                    Ok(planes) => planes,
                    Err(err) => {
                        warn!("Failed to query surface planes: {}", err);
                        return;
                    }
                };

                // Using an overlay plane on a nvidia card breaks
                if driver
                    .name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("nvidia")
                    || driver
                        .description()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains("nvidia")
                {
                    planes.overlay = vec![];
                }

                let compositor = GbmDrmCompositor::new(
                    &output,
                    drm_surface,
                    Some(planes),
                    gbm_allocator,
                    device.gbm.clone(),
                    SUPPORTED_FORMATS,
                    render_formats,
                    device.drm.cursor_size(),
                    Some(device.gbm.clone()),
                )
                .unwrap();

                let pointer_texture = TextureBuffer::from_memory(
                    &mut renderer,
                    CURSOR_DATA,
                    Fourcc::Abgr8888,
                    (64, 64),
                    false,
                    1,
                    Transform::Normal,
                    None,
                )
                .unwrap();

                let surface = Surface {
                    _device_id: node,
                    _render_node: device.render_node,
                    global,
                    compositor,
                    _output: output.clone(),
                    pointer_texture,
                };

                for workspace in self.workspaces.iter() {
                    workspace.remove_outputs();
                    workspace.add_output(output.clone())
                }

                device.surfaces.insert(crtc, surface);

                self.render(node, crtc).ok();
            }
            DrmScanEvent::Disconnected {
                crtc: Some(crtc), ..
            } => {
                device.surfaces.remove(&crtc);
            }
            _ => {}
        }
    }
}

impl MagmaState<UdevData> {
    pub fn render(&mut self, node: DrmNode, crtc: crtc::Handle) -> Result<bool, SwapBuffersError> {
        let device = self.backend_data.devices.get_mut(&node).unwrap();
        let surface = device.surfaces.get_mut(&crtc).unwrap();
        let mut renderer = self
            .backend_data
            .gpus
            .single_renderer(&device.render_node)
            .unwrap();
        let output = self.workspaces.current().outputs().next().unwrap();

        let mut renderelements: Vec<CustomRenderElements<MultiRenderer<_, _>>> = vec![];

        renderelements.append(&mut vec![
            CustomRenderElements::<MultiRenderer<_, _>>::from(
                TextureRenderElement::from_texture_buffer(
                    self.pointer_location.to_physical(Scale::from(1.0)),
                    &surface.pointer_texture,
                    None,
                    None,
                    None,
                ),
            ),
        ]);

        let layer_map = layer_map_for_output(output);
        let (lower, upper): (Vec<&LayerSurface>, Vec<&LayerSurface>) = layer_map
            .layers()
            .rev()
            .partition(|s| matches!(s.layer(), Layer::Background | Layer::Bottom));

        renderelements.extend(
            upper
                .into_iter()
                .filter_map(|surface| {
                    layer_map
                        .layer_geometry(surface)
                        .map(|geo| (geo.loc, surface))
                })
                .flat_map(|(loc, surface)| {
                    AsRenderElements::<MultiRenderer<_, _>>::render_elements::<
                        WaylandSurfaceRenderElement<MultiRenderer<_, _>>,
                    >(
                        surface,
                        &mut renderer,
                        loc.to_physical_precise_round(1),
                        Scale::from(1.0),
                    )
                    .into_iter()
                    .map(CustomRenderElements::Surface)
                }),
        );

        renderelements.extend(self.workspaces.current().render_elements(&mut renderer));

        renderelements.extend(
            lower
                .into_iter()
                .filter_map(|surface| {
                    layer_map
                        .layer_geometry(surface)
                        .map(|geo| (geo.loc, surface))
                })
                .flat_map(|(loc, surface)| {
                    AsRenderElements::<MultiRenderer<_, _>>::render_elements::<
                        WaylandSurfaceRenderElement<MultiRenderer<_, _>>,
                    >(
                        surface,
                        &mut renderer,
                        loc.to_physical_precise_round(1),
                        Scale::from(1.0),
                    )
                    .into_iter()
                    .map(CustomRenderElements::Surface)
                }),
        );

        let frame_result = surface
            .compositor
            .render_frame::<_, _, GlesTexture>(&mut renderer, &renderelements, [0.1, 0.1, 0.1, 1.0])
            .unwrap();

        let rendered = frame_result.damage.is_some();
        let mut result = Ok(rendered);
        if rendered {
            let queueresult = surface
                .compositor
                .queue_frame(())
                .map_err(Into::<SwapBuffersError>::into);
            if let Err(queueresult) = queueresult {
                result = Err(queueresult);
            }
        }

        let reschedule = match &result {
            Ok(has_rendered) => !has_rendered,
            Err(err) => {
                warn!("Error during rendering: {:?}", err);
                match err {
                    SwapBuffersError::AlreadySwapped => false,
                    SwapBuffersError::TemporaryFailure(err) => !matches!(
                        err.downcast_ref::<DrmError>(),
                        Some(&DrmError::DeviceInactive)
                            | Some(&DrmError::Access {
                                source: SystemError::PermissionDenied,
                                ..
                            })
                    ),
                    SwapBuffersError::ContextLost(err) => {
                        warn!("Rendering loop lost: {}", err);
                        false
                    }
                }
            }
        };

        if reschedule {
            let output_refresh = match output.current_mode() {
                Some(mode) => mode.refresh,
                None => return result,
            };
            // If reschedule is true we either hit a temporary failure or more likely rendering
            // did not cause any damage on the output. In this case we just re-schedule a repaint
            // after approx. one frame to re-test for damage.
            let reschedule_duration =
                Duration::from_millis((1_000_000f32 / output_refresh as f32) as u64);
            trace!(
                "reschedule repaint timer with delay {:?} on {:?}",
                reschedule_duration,
                crtc,
            );
            let timer = Timer::from_duration(reschedule_duration);
            self.loop_handle
                .insert_source(timer, move |_, _, data| {
                    data.state.render(node, crtc).ok();
                    TimeoutAction::Drop
                })
                .expect("failed to schedule frame timer");
        }

        self.workspaces.current().windows().for_each(|window| {
            window.send_frame(
                output,
                self.start_time.elapsed(),
                Some(Duration::ZERO),
                |_, _| Some(output.clone()),
            );
        });
        result
    }
}
