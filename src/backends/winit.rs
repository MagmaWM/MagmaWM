use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitError, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    desktop::space::SpaceElement,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        wayland_server::Display,
    },
    utils::{Rectangle, Transform},
};

pub struct WinitData {
    backend: WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: OutputDamageTracker,
}

impl Backend for WinitData {
    fn seat_name(&self) -> String {
        "winit".to_string()
    }
}
use crate::state::{Backend, CalloopData, MagmaState};

pub fn init_winit() {
    let mut event_loop: EventLoop<CalloopData<WinitData>> = EventLoop::try_new().unwrap();

    let mut display: Display<MagmaState<WinitData>> = Display::new().unwrap();

    let (backend, mut winit) = winit::init().unwrap();

    let mode = Mode {
        size: backend.window_size().physical_size,
        refresh: 60_000,
    };

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "MagmaEWM".into(),
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

    let winitdata = WinitData {
        backend,
        damage_tracker: damage_tracked_renderer,
    };
    let state = MagmaState::new(
        event_loop.handle(),
        event_loop.get_signal(),
        &mut display,
        winitdata,
    );

    let mut data = CalloopData { display, state };

    let state = &mut data.state;

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

    std::process::Command::new("alacritty")
        .spawn()
        .expect("this should work");

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
    let display = &mut data.display;
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
        }
        WinitEvent::Input(event) => state.process_input_event(event),
        _ => (),
    });

    let winitdata = &mut state.backend_data;

    if let Err(WinitError::WindowClosed) = res {
        // Stop the loop
        state.loop_signal.stop();
    } else {
        res.unwrap();
    }

    *full_redraw = full_redraw.saturating_sub(1);

    let size = winitdata.backend.window_size().physical_size;
    let damage = Rectangle::from_loc_and_size((0, 0), size);

    winitdata.backend.bind().unwrap();

    let mut renderelements: Vec<WaylandSurfaceRenderElement<_>> = vec![];

    let workspace = state.workspaces.current_mut();
    let output = workspace.outputs().next().unwrap();

    renderelements.extend(workspace.render_elements(winitdata.backend.renderer()));

    winitdata
        .damage_tracker
        .render_output(
            winitdata.backend.renderer(),
            0,
            &renderelements,
            [0.1, 0.1, 0.1, 1.0],
        )
        .unwrap();

    winitdata.backend.submit(Some(&[damage])).unwrap();

    workspace.windows().for_each(|window| {
        window.send_frame(
            output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });

    workspace.windows().for_each(|e| e.refresh());
    display.flush_clients().unwrap();
    state.popup_manager.cleanup();
}
