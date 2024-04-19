use smithay::{
    backend::{
        input::{
            self, AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend,
            InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
            PointerMotionEvent,
        },
        libinput::LibinputInputBackend,
    },
    input::{
        keyboard::{xkb, FilterResult},
        pointer::{AxisFrame, ButtonEvent, MotionEvent, RelativeMotionEvent},
    },
    reexports::input::Led,
    utils::{Logical, Point, SERIAL_COUNTER},
};
use tracing::info;

use crate::{
    backends::udev::UdevData,
    config::Action,
    state::{Backend, MagmaState, CONFIG},
    utils::focus::FocusTarget,
    utils::process,
};

impl MagmaState<UdevData> {
    pub fn process_input_event_udev(
        &mut self,
        event: InputEvent<LibinputInputBackend>,
    ) -> Option<i32> {
        match event {
            InputEvent::Keyboard { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                if let Some(action) = self.seat.get_keyboard().unwrap().input(
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    time,
                    |state, modifiers, handle| {
                        let mut leds = Led::empty();
                        if modifiers.caps_lock {
                            leds.insert(Led::CAPSLOCK);
                        }
                        if modifiers.num_lock {
                            leds.insert(Led::NUMLOCK);
                        }
                        event.device().led_update(leds);
                        #[cfg(feature = "debug")]
                        if state.debug.egui.wants_keyboard() {
                            state.debug.egui.handle_keyboard(
                                &handle,
                                event.state() == KeyState::Pressed,
                                *modifiers,
                            );
                            return FilterResult::Intercept(None);
                        }
                        for (binding, action) in CONFIG.keybindings.iter() {
                            if event.state() == KeyState::Pressed
                                && binding.modifiers == *modifiers
                                && handle.raw_syms().contains(&binding.key)
                            {
                                return FilterResult::Intercept(Some(action.clone()));
                            } else if (xkb::keysyms::KEY_XF86Switch_VT_1
                                ..=xkb::keysyms::KEY_XF86Switch_VT_12)
                                .contains(&handle.modified_sym().raw())
                            {
                                // VTSwitch
                                let vt = (handle.modified_sym().raw()
                                    - xkb::keysyms::KEY_XF86Switch_VT_1
                                    + 1) as i32;
                                return FilterResult::Intercept(Some(Action::VTSwitch(vt)));
                            }
                        }
                        FilterResult::Forward
                    },
                ) {
                    match action {
                        Some(Action::VTSwitch(vt)) => return Some(vt),
                        Some(action) => self.handle_action(action),
                        None => {}
                    }
                };
                None
            }
            InputEvent::DeviceAdded { mut device } => {
                device.config_tap_set_enabled(true).ok();
                device.config_tap_set_drag_enabled(true).ok();
                None
            }
            event => {
                self.process_input_event(event);
                None
            }
        }
    }
}

impl<BackendData: Backend> MagmaState<BackendData> {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            #[cfg(feature = "debug")]
            InputEvent::DeviceAdded { device } => {
                self.debug.egui.handle_device_added(&device);
            }
            #[cfg(feature = "debug")]
            InputEvent::DeviceRemoved { device } => {
                self.debug.egui.handle_device_removed(&device);
            }
            InputEvent::Keyboard { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);

                if let Some(Some(action)) = self.seat.get_keyboard().unwrap().input(
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    time,
                    |state, modifiers, handle| {
                        #[cfg(feature = "debug")]
                        if state.debug.egui.wants_keyboard() {
                            state.debug.egui.handle_keyboard(
                                &handle,
                                event.state() == KeyState::Pressed,
                                *modifiers,
                            );
                            return FilterResult::Intercept(None);
                        }
                        for (binding, action) in CONFIG.keybindings.iter() {
                            if event.state() == KeyState::Pressed
                                && binding.modifiers == *modifiers
                                && handle.raw_syms().contains(&binding.key)
                            {
                                return FilterResult::Intercept(Some(action.clone()));
                            }
                        }
                        FilterResult::Forward
                    },
                ) {
                    self.handle_action(action);
                };
            }
            InputEvent::PointerMotion { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let delta = (event.delta_x(), event.delta_y()).into();
                self.pointer_location += delta;

                // clamp to screen limits
                // this event is never generated by winit
                self.pointer_location = self.clamp_coords(self.pointer_location);

                let under = self.surface_under();

                self.set_input_focus_auto();

                if let Some(ptr) = self.seat.get_pointer() {
                    ptr.motion(
                        self,
                        under.clone(),
                        &MotionEvent {
                            location: self.pointer_location,
                            serial,
                            time: event.time_msec(),
                        },
                    );

                    ptr.relative_motion(
                        self,
                        under,
                        &RelativeMotionEvent {
                            delta,
                            delta_unaccel: event.delta_unaccel(),
                            utime: event.time(),
                        },
                    )
                }
                #[cfg(feature = "debug")]
                self.debug
                    .egui
                    .handle_pointer_motion(self.pointer_location.to_i32_round())
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.workspaces.current().outputs().next().unwrap().clone();

                let output_geo = self.workspaces.current().output_geometry(&output).unwrap();

                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial = SERIAL_COUNTER.next_serial();

                let pointer = self.seat.get_pointer().unwrap();

                self.pointer_location = self.clamp_coords(pos);

                let under = self.surface_under();

                self.set_input_focus_auto();

                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                #[cfg(feature = "debug")]
                self.debug
                    .egui
                    .handle_pointer_motion(self.pointer_location.to_i32_round())
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer().unwrap();

                let serial = SERIAL_COUNTER.next_serial();

                let button = event.button_code();

                let button_state = event.state();

                self.set_input_focus_auto();
                #[cfg(feature = "debug")]
                if self.debug.egui.wants_pointer() {
                    if let Some(button) = event.button() {
                        self.debug
                            .egui
                            .handle_pointer_button(button, event.state() == ButtonState::Pressed);
                    }
                    return;
                }
                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
            }
            InputEvent::PointerAxis { event, .. } => {
                #[cfg(feature = "debug")]
                if self.debug.egui.wants_pointer() {
                    self.debug.egui.handle_pointer_axis(
                        event
                            .amount_v120(Axis::Horizontal)
                            .or_else(|| event.amount(Axis::Horizontal).map(|x| x * 3.0))
                            .unwrap_or(0.0),
                        event
                            .amount_v120(Axis::Vertical)
                            .or_else(|| event.amount(Axis::Vertical).map(|x| x * 3.0))
                            .unwrap_or(0.0),
                    );
                    return;
                }
                let horizontal_amount =
                    event.amount(input::Axis::Horizontal).unwrap_or_else(|| {
                        event.amount_v120(input::Axis::Horizontal).unwrap_or(0.0) * 3.0
                    });
                let vertical_amount = event.amount(input::Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(input::Axis::Vertical).unwrap_or(0.0) * 3.0
                });
                let horizontal_amount_discrete = event.amount_v120(input::Axis::Horizontal);
                let vertical_amount_discrete = event.amount_v120(input::Axis::Vertical);

                {
                    let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
                    if horizontal_amount != 0.0 {
                        frame = frame.value(Axis::Horizontal, horizontal_amount);
                        if let Some(discrete) = horizontal_amount_discrete {
                            frame = frame.v120(Axis::Horizontal, discrete as i32);
                        }
                    } else if event.source() == AxisSource::Finger {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if vertical_amount != 0.0 {
                        frame = frame.value(Axis::Vertical, vertical_amount);
                        if let Some(discrete) = vertical_amount_discrete {
                            frame = frame.v120(Axis::Vertical, discrete as i32);
                        }
                    } else if event.source() == AxisSource::Finger {
                        frame = frame.stop(Axis::Vertical);
                    }
                    self.seat.get_pointer().unwrap().axis(self, frame);
                }
            }
            _ => {}
        }
    }

    fn clamp_coords(&self, pos: Point<f64, Logical>) -> Point<f64, Logical> {
        if self.workspaces.current().outputs().next().is_none() {
            return pos;
        }

        let (pos_x, pos_y) = pos.into();
        let (max_x, max_y) = self
            .workspaces
            .current()
            .output_geometry(self.workspaces.current().outputs().next().unwrap())
            .unwrap()
            .size
            .into();
        let clamped_x = pos_x.max(0.0).min(max_x as f64);
        let clamped_y = pos_y.max(0.0).min(max_y as f64);
        (clamped_x, clamped_y).into()
    }

    pub fn set_input_focus(&mut self, target: FocusTarget) {
        let keyboard = self.seat.get_keyboard().unwrap();
        let serial = SERIAL_COUNTER.next_serial();
        keyboard.set_focus(self, Some(target), serial);
    }

    pub fn set_input_focus_auto(&mut self) {
        let under = self.surface_under();
        if let Some(d) = under {
            self.set_input_focus(d.0);
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.loop_signal.stop(),
            #[cfg(feature = "debug")]
            Action::Debug => self.debug.active = !self.debug.active,
            Action::Close => {
                if let Some(d) = self
                    .workspaces
                    .current()
                    .window_under(self.pointer_location)
                {
                    d.0.toplevel().unwrap().send_close()
                }
            }
            Action::Workspace(id) => {
                self.workspaces.activate(id);
                self.set_input_focus_auto();
            }
            Action::MoveWindow(id) => {
                let window = self
                    .workspaces
                    .current()
                    .window_under(self.pointer_location)
                    .map(|d| d.0.clone());

                if let Some(window) = window {
                    self.workspaces.move_window_to_workspace(&window, id);
                }
            }
            Action::MoveAndSwitch(u8) => {
                self.handle_action(Action::MoveWindow(u8));
                self.handle_action(Action::Workspace(u8));
            }
            Action::ToggleWindowFloating => todo!(),
            Action::Spawn(command) => {
                process::spawn(&command);
            }
            Action::VTSwitch(_) => {
                info!("VTSwitch is not used in Winit backend.")
            }
        }
    }
}
