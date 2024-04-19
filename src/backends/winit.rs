use std::time::Duration;

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::EGLDevice,
        renderer::{
            damage::OutputDamageTracker, element::AsRenderElements, glow::GlowRenderer, ImportDma,
            ImportEgl,
        },
        winit::{self, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    delegate_dmabuf,
    desktop::{layer_map_for_output, space::SpaceElement, LayerSurface},
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        wayland_server::{Display, DisplayHandle},
        winit::platform::pump_events::PumpStatus,
    },
    utils::{Rectangle, Scale, Transform},
    wayland::{
        dmabuf::{
            DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufHandler, DmabufState,
            ImportNotifier,
        },
        shell::wlr_layer::Layer,
    },
};
use tracing::{info, warn};

use crate::utils::process;

pub struct WinitData {
    backend: WinitGraphicsBackend<GlowRenderer>,
    damage_tracker: OutputDamageTracker,
    dmabuf_state: (DmabufState, DmabufGlobal, Option<DmabufFeedback>),
}

impl DmabufHandler for MagmaState<WinitData> {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.backend_data.dmabuf_state.0
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) {
        if self
            .backend_data
            .backend
            .renderer()
            .import_dmabuf(&dmabuf, None)
            .is_ok()
        {
            let _ = notifier.successful::<MagmaState<WinitData>>();
        } else {
            notifier.failed();
        }
    }
}
delegate_dmabuf!(MagmaState<WinitData>);

impl Backend for WinitData {
    fn seat_name(&self) -> String {
        "winit".to_string()
    }
}
use crate::{
    state::{Backend, CalloopData, MagmaState, CONFIG},
    utils::render::{border::BorderShader, init_shaders, CustomRenderElements},
};

pub fn init_winit() {
    let mut event_loop: EventLoop<CalloopData<WinitData>> = EventLoop::try_new().unwrap();

    let display: Display<MagmaState<WinitData>> = Display::new().unwrap();

    let (mut backend, mut winit) = winit::init::<GlowRenderer>().unwrap();

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "MagmaWM".into(),
            model: "Winit".into(),
        },
    );
    let _global = output.create_global::<MagmaState<WinitData>>(&display.handle());
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    let damage_tracked_renderer = OutputDamageTracker::from_output(&output);

    let render_node = EGLDevice::device_for_display(backend.renderer().egl_context().display())
        .and_then(|device| device.try_get_render_node());

    let dmabuf_default_feedback = match render_node {
        Ok(Some(node)) => {
            let dmabuf_formats = backend.renderer().dmabuf_formats().collect::<Vec<_>>();
            let dmabuf_default_feedback = DmabufFeedbackBuilder::new(node.dev_id(), dmabuf_formats)
                .build()
                .unwrap();
            Some(dmabuf_default_feedback)
        }
        Ok(None) => {
            warn!("failed to query render node, dmabuf will use v3");
            None
        }
        Err(err) => {
            warn!(?err, "failed to egl device for display, dmabuf will use v3");
            None
        }
    };

    // if we failed to build dmabuf feedback we fall back to dmabuf v3
    // Note: egl on Mesa requires either v4 or wl_drm (initialized with bind_wl_display)
    let dmabuf_state = if let Some(default_feedback) = dmabuf_default_feedback {
        let mut dmabuf_state = DmabufState::new();
        let dmabuf_global = dmabuf_state
            .create_global_with_default_feedback::<MagmaState<WinitData>>(
                &display.handle(),
                &default_feedback,
            );
        (dmabuf_state, dmabuf_global, Some(default_feedback))
    } else {
        let dmabuf_formats = backend.renderer().dmabuf_formats().collect::<Vec<_>>();
        let mut dmabuf_state = DmabufState::new();
        let dmabuf_global =
            dmabuf_state.create_global::<MagmaState<WinitData>>(&display.handle(), dmabuf_formats);
        (dmabuf_state, dmabuf_global, None)
    };

    if backend
        .renderer()
        .bind_wl_display(&display.handle())
        .is_ok()
    {
        info!("EGL hardware-acceleration enabled");
    };

    let winitdata = WinitData {
        backend,
        damage_tracker: damage_tracked_renderer,
        dmabuf_state,
    };
    let display_handle: DisplayHandle = display.handle().clone();
    let state = MagmaState::new(
        event_loop.handle(),
        event_loop.get_signal(),
        display,
        winitdata,
    );

    let mut data = CalloopData {
        display_handle,
        state,
    };

    let state = &mut data.state;
    init_shaders(state.backend_data.backend.renderer());
    // map output to every workspace
    for workspace in state.workspaces.iter() {
        workspace.add_output(output.clone());
    }

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    let mut full_redraw = 0u8;

    let timer = Timer::immediate();

    event_loop
        .handle()
        .insert_source(timer, move |_, _, data| {
            winit_dispatch(&mut winit, data, &output, &mut full_redraw);
            TimeoutAction::ToDuration(Duration::from_millis(16))
        })
        .unwrap();

    for command in &CONFIG.autostart {
        process::spawn(command);
    }

    event_loop
        .run(None, &mut data, move |_| {
            // Magma is running
        })
        .unwrap();
}

pub fn winit_dispatch(
    winit: &mut WinitEventLoop,
    data: &mut CalloopData<WinitData>,
    output: &Output,
    full_redraw: &mut u8,
) {
    let state = &mut data.state;

    let res = winit.dispatch_new_events(|event| match event {
        WinitEvent::Resized { size, .. } => {
            output.change_current_state(
                Some(Mode {
                    size,
                    refresh: 60_000,
                }),
                None,
                None,
                None,
            );
            layer_map_for_output(output).arrange();
        }
        WinitEvent::Input(event) => state.process_input_event(event),
        _ => (),
    });

    let winitdata = &mut state.backend_data;

    if let PumpStatus::Exit(_) = res {
        // Stop the loop
        state.loop_signal.stop();
    }

    *full_redraw = full_redraw.saturating_sub(1);
    #[cfg(feature = "debug")]
    state.debug.fps.start();

    let size = winitdata.backend.window_size();
    let damage = Rectangle::from_loc_and_size((0, 0), size);

    let mut renderelements: Vec<CustomRenderElements<_>> = vec![];
    let workspace = state.workspaces.current_mut();
    let output = workspace.outputs().next().unwrap();
    #[cfg(feature = "debug")]
    if state.debug.active {
        renderelements.push(
            state
                .debug
                .global_ui(
                    None,
                    output,
                    &state.seat,
                    winitdata.backend.renderer(),
                    Rectangle::from_loc_and_size(
                        (0, 0),
                        output.current_mode().unwrap().size.to_logical(1),
                    ),
                    1.0,
                    0.8,
                )
                .unwrap()
                .into(),
        );
    }
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
                AsRenderElements::<GlowRenderer>::render_elements::<CustomRenderElements<_>>(
                    surface,
                    winitdata.backend.renderer(),
                    loc.to_physical_precise_round(1),
                    Scale::from(1.0),
                    1.0,
                )
            }),
    );

    renderelements.extend(workspace.render_elements(winitdata.backend.renderer()));

    renderelements.extend(
        lower
            .into_iter()
            .filter_map(|surface| {
                layer_map
                    .layer_geometry(surface)
                    .map(|geo| (geo.loc, surface))
            })
            .flat_map(|(loc, surface)| {
                AsRenderElements::<GlowRenderer>::render_elements::<CustomRenderElements<_>>(
                    surface,
                    winitdata.backend.renderer(),
                    loc.to_physical_precise_round(1),
                    Scale::from(1.0),
                    1.0,
                )
            }),
    );
    #[cfg(feature = "debug")]
    state.debug.fps.elements();
    winitdata.backend.bind().unwrap();
    winitdata
        .damage_tracker
        .render_output(
            winitdata.backend.renderer(),
            0,
            &renderelements,
            [0.1, 0.1, 0.1, 1.0],
        )
        .unwrap();
    #[cfg(feature = "debug")]
    state.debug.fps.render();
    winitdata.backend.submit(Some(&[damage])).unwrap();
    #[cfg(feature = "debug")]
    state.debug.fps.displayed();
    workspace.windows().for_each(|window| {
        window.send_frame(
            output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });

    workspace.windows().for_each(|e| e.refresh());
    data.display_handle.flush_clients().unwrap();
    state.popup_manager.cleanup();
    BorderShader::cleanup(winitdata.backend.renderer());
}
