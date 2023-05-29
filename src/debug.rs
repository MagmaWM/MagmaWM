use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    time::{Duration, Instant},
};

use egui::{
    plot::{Bar, BarChart, Legend, Plot, Corner},
    Color32, Context, FullOutput,
};
use egui_glow::Painter;
use smithay::{
    backend::{
        allocator::Fourcc,
        drm::DrmNode,
        renderer::{
            element::texture::{TextureRenderBuffer, TextureRenderElement},
            gles::{GlesError, GlesTexture},
            glow::GlowRenderer,
            Bind, Frame as RenderFrame, Offscreen, Renderer, Unbind,
        },
    },
    input::{keyboard::xkb, Seat},
    output::Output,
    reexports::wayland_server::Resource,
    utils::{Logical, Rectangle, Transform},
    wayland::{compositor::with_states, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{
    state::{Backend, MagmaState},
    utils::focus::FocusTarget,
};

pub const ELEMENTS_COLOR: Color32 = Color32::from_rgb(173, 216, 230);
pub const RENDER_COLOR: Color32 = Color32::from_rgb(255, 127, 80);
pub const SCREENCOPY_COLOR: Color32 = Color32::from_rgb(255, 255, 153);
pub const DISPLAY_COLOR: Color32 = Color32::from_rgb(152, 251, 152);
const VENDORS: [(&str, &str); 3] = [("0x10de", "nvidia"), ("0x1002", "amd"), ("0x8086", "intel")];

struct GlState {
    painter: Painter,
    render_buffer: TextureRenderBuffer<GlesTexture>,
}
type UserDataType = Rc<RefCell<GlState>>;

#[derive(Default)]
pub struct MagmaDebug {
    ctx: Context,
    pub active: bool,
    pub fps: Fps,
}

impl MagmaDebug {
    pub fn render(
        &mut self,
        ui: impl FnOnce(&Context),
        renderer: &mut GlowRenderer,
        area: Rectangle<i32, Logical>,
        scale: i32,
        alpha: f32,
    ) -> Result<TextureRenderElement<GlesTexture>, GlesError> {
        let user_data = renderer.egl_context().user_data();
        if user_data.get::<UserDataType>().is_none() {
            let painter = {
                let mut frame = renderer.render(
                    area.size.to_physical(scale),
                    smithay::utils::Transform::Normal,
                )?;
                frame
                    .with_context(|context| Painter::new(context.clone(), "", None))?
                    .map_err(|_| GlesError::ShaderCompileError)?
            };
            let render_buffer = {
                let render_texture = renderer
                    .create_buffer(
                        Fourcc::Abgr8888,
                        area.size
                            .to_buffer(scale, smithay::utils::Transform::Normal),
                    )
                    .expect("Failed to create buffer");
                TextureRenderBuffer::from_texture(
                    renderer,
                    render_texture,
                    scale,
                    Transform::Flipped180,
                    None,
                )
            };
            renderer.egl_context().user_data().insert_if_missing(|| {
                UserDataType::new(RefCell::new(GlState {
                    painter,
                    render_buffer,
                }))
            });
        }

        let gl_state = renderer
            .egl_context()
            .user_data()
            .get::<UserDataType>()
            .unwrap()
            .clone();
        let mut borrow = gl_state.borrow_mut();
        let &mut GlState {
            ref mut painter,
            ref mut render_buffer,
            ..
        } = &mut *borrow;

        let input = egui::RawInput::default();
        let FullOutput {
            shapes,
            textures_delta,
            ..
        } = self.ctx.run(input, ui);

        render_buffer.render().draw(|tex| {
            renderer.bind(tex.clone())?;
            let physical_area = area.to_physical(scale);
            {
                let mut frame = renderer.render(physical_area.size, Transform::Normal)?;
                frame.clear([0.0, 0.0, 0.0, 0.0], &[physical_area])?;
                painter.paint_and_update_textures(
                    [physical_area.size.w as u32, physical_area.size.h as u32],
                    scale as f32,
                    &self.ctx.tessellate(shapes),
                    &textures_delta,
                );
            }
            renderer.unbind()?;

            let used = self.ctx.used_rect();
            let margin = self.ctx.style().visuals.clip_rect_margin.ceil() as i32;
            let window_shadow = self.ctx.style().visuals.window_shadow.extrusion.ceil() as i32;
            let popup_shadow = self.ctx.style().visuals.popup_shadow.extrusion.ceil() as i32;
            let offset = margin + Ord::max(window_shadow, popup_shadow);
            Result::<_, GlesError>::Ok(vec![Rectangle::<i32, Logical>::from_extemities(
                (
                    (used.min.x.floor() as i32).saturating_sub(offset),
                    (used.min.y.floor() as i32).saturating_sub(offset),
                ),
                (
                    (used.max.x.ceil() as i32) + (offset * 2),
                    (used.max.y.ceil() as i32) + (offset * 2),
                ),
            )
            .to_buffer(scale, Transform::Flipped180, &area.size)])
        })?;

        Ok(TextureRenderElement::from_texture_render_buffer(
            area.loc.to_f64().to_physical(scale as f64),
            render_buffer,
            Some(alpha),
            None,
            None,
        ))
    }

    pub fn global_ui<BackendData: Backend>(
        &mut self,
        gpu: Option<&DrmNode>,
        output: &Output,
        seat: &Seat<MagmaState<BackendData>>,
        renderer: &mut GlowRenderer,
        area: Rectangle<i32, Logical>,
        scale: i32,
        alpha: f32,
    ) -> Result<TextureRenderElement<GlesTexture>, GlesError> {
        let (max, min, avg, avg_fps, potential_fps) = (
            self.fps.max_frametime().as_secs_f64(),
            self.fps.min_frametime().as_secs_f64(),
            self.fps.avg_frametime().as_secs_f64(),
            self.fps.avg_fps(),
            self.fps.potential_fps(),
        );
        let (bars_elements, bars_render, bars_screencopy, bars_displayed): (
            Vec<Bar>,
            Vec<Bar>,
            Vec<Bar>,
            Vec<Bar>,
        ) = self
            .fps
            .frames
            .iter()
            .enumerate()
            .map(|(i, frame)| {
                (
                    Bar::new(i as f64, frame.duration_elements.as_secs_f64()).fill(ELEMENTS_COLOR),
                    Bar::new(i as f64, frame.duration_render.as_secs_f64()).fill(RENDER_COLOR),
                    Bar::new(
                        i as f64,
                        frame
                            .duration_screencopy
                            .as_ref()
                            .map(|val| val.as_secs_f64())
                            .unwrap_or(0.0),
                    )
                    .fill(SCREENCOPY_COLOR),
                    Bar::new(i as f64, frame.duration_displayed.as_secs_f64()).fill(DISPLAY_COLOR),
                )
            })
            .fold((vec![], vec![], vec![], vec![]), |mut out, cur| {
                out.0.push(cur.0);
                out.1.push(cur.1);
                out.2.push(cur.2);
                out.3.push(cur.3);
                out
            });
        self.render(
            |ctx| {
                egui::Area::new("main")
                    .anchor(egui::Align2::LEFT_TOP, (10.0, 10.0))
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "MagmaWM version {}",
                            std::env!("CARGO_PKG_VERSION")
                        ));
                        if let Some(hash) = std::option_env!("GIT_HASH").and_then(|x| x.get(0..10))
                        {
                            ui.label(format!("git: {hash}"));
                        }
                        ui.set_max_width(300.0);
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "FPS: {:>7.3}/{:>7.3}",
                                avg_fps, potential_fps
                            ))
                            .heading(),
                        );
                        ui.label("Frame Times:");
                        ui.label(egui::RichText::new(format!("avg: {:>7.6}", avg)).code());
                        ui.label(egui::RichText::new(format!("min: {:>7.6}", min)).code());
                        ui.label(egui::RichText::new(format!("max: {:>7.6}", max)).code());
                        let elements_chart = BarChart::new(bars_elements)
                            .vertical()
                            .name("elements")
                            .color(ELEMENTS_COLOR);
                        let render_chart = BarChart::new(bars_render)
                            .stack_on(&[&elements_chart])
                            .vertical()
                            .name("render")
                            .color(RENDER_COLOR);
                        let screencopy_chart = BarChart::new(bars_screencopy)
                            .stack_on(&[&elements_chart, &render_chart])
                            .vertical()
                            .name("screencopy")
                            .color(SCREENCOPY_COLOR);
                        let display_chart = BarChart::new(bars_displayed)
                            .stack_on(&[&elements_chart, &render_chart, &screencopy_chart])
                            .vertical()
                            .name("display")
                            .color(DISPLAY_COLOR);

                        Plot::new("FPS")
                            .legend(Legend::default().position(Corner::LeftBottom).background_alpha(0.0))
                            .height(100.0)
                            .show_x(false)
                            .show(ui, |plot_ui| {
                                plot_ui.bar_chart(elements_chart);
                                plot_ui.bar_chart(render_chart);
                                plot_ui.bar_chart(screencopy_chart);
                                plot_ui.bar_chart(display_chart);
                            });
                        ui.separator();
                        if let Some(gpu) = gpu {
                            ui.label(egui::RichText::new(format!("gpu: {}", gpu)).strong());
                            if let Ok(vendor) = std::fs::read_to_string(format!(
                                "/sys/class/drm/{}/device/vendor",
                                gpu
                            )) {
                                ui.label(egui::RichText::new(format!(
                                    "Vendor: {}",
                                    VENDORS
                                        .iter()
                                        .find(|v| v.0 == vendor.trim())
                                        .and_then(|v| Some(v.1))
                                        .unwrap_or(&"Unknown")
                                )));
                            }
                            ui.label(format!(
                                "Resolution: {}x{}",
                                output.current_mode().unwrap().size.w,
                                output.current_mode().unwrap().size.h
                            ));
                            ui.label(format!(
                                "Refresh Rate: {}hz",
                                output.current_mode().unwrap().refresh / 1000
                            ));
                            ui.separator();
                        }
                        ui.label(egui::RichText::new(format!("\t{}", seat.name())).strong());
                        if let Some(ptr) = seat.get_pointer() {
                            egui::Frame::none()
                                .fill(egui::Color32::DARK_GRAY)
                                .rounding(5.)
                                .inner_margin(10.)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Pos: {:?}",
                                            ptr.current_location()
                                        ))
                                        .code(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Focus: {}",
                                            format_focus(ptr.current_focus())
                                        ))
                                        .code(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Grabbed: {:?}",
                                            ptr.is_grabbed()
                                        ))
                                        .code(),
                                    );
                                });
                        }
                        if let Some(kbd) = seat.get_keyboard() {
                            egui::Frame::none()
                                .fill(egui::Color32::DARK_GRAY)
                                .rounding(5.)
                                .inner_margin(10.)
                                .show(ui, |ui| {
                                    let mut keysyms = "".to_string();
                                    kbd.with_pressed_keysyms(|syms| {
                                        keysyms = format!(
                                            "Keys: {}",
                                            syms.into_iter()
                                                .map(|k| xkb::keysym_get_name(k.modified_sym()))
                                                .fold(String::new(), |mut list, val| {
                                                    list.push_str(&format!("{}, ", val));
                                                    list
                                                })
                                        )
                                    });
                                    keysyms.truncate(keysyms.len().saturating_sub(2));
                                    ui.label(egui::RichText::new(keysyms).code());

                                    let mods = kbd.modifier_state();
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Mods: Ctrl {} / Alt {} / Logo {} / Shift {}",
                                            mods.ctrl, mods.alt, mods.logo, mods.shift,
                                        ))
                                        .code(),
                                    );

                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Focus: {}",
                                            format_focus(kbd.current_focus())
                                        ))
                                        .code(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Grabbed: {:?}",
                                            kbd.is_grabbed()
                                        ))
                                        .code(),
                                    );
                                });
                        }
                    });
            },
            renderer,
            area,
            scale,
            alpha,
        )
    }
}

fn format_focus(focus: Option<FocusTarget>) -> String {
    if let Some(focus) = focus {
        match focus {
            FocusTarget::Window(w) => format!(
                "Window {} ({})",
                w.toplevel().wl_surface().id().protocol_id(),
                with_states(w.toplevel().wl_surface(), |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .title
                        .clone()
                        .unwrap_or_default()
                })
            ),
            FocusTarget::LayerSurface(l) => {
                format!("LayerSurface {}", l.wl_surface().id().protocol_id())
            }
            FocusTarget::Popup(p) => format!("Popup {}", p.wl_surface().id().protocol_id()),
        }
    } else {
        format!("None")
    }
}

#[derive(Default)]
pub struct Fps {
    current_frame: Option<PendingFrame>,
    frames: VecDeque<Frame>,
}

struct PendingFrame {
    start: Instant,
    duration_elements: Option<Duration>,
    duration_render: Option<Duration>,
    duration_screencopy: Option<Duration>,
    duration_displayed: Option<Duration>,
}

pub struct Frame {
    pub start: Instant,
    pub duration_elements: Duration,
    pub duration_render: Duration,
    pub duration_screencopy: Option<Duration>,
    pub duration_displayed: Duration,
}

impl Frame {
    fn frame_time(&self) -> Duration {
        self.duration_elements
            + self.duration_render
            + self.duration_screencopy.clone().unwrap_or(Duration::ZERO)
    }

    fn time_to_display(&self) -> Duration {
        self.duration_elements
            + self.duration_render
            + self.duration_screencopy.clone().unwrap_or(Duration::ZERO)
            + self.duration_displayed
    }
}

impl From<PendingFrame> for Frame {
    fn from(pending: PendingFrame) -> Self {
        Frame {
            start: pending.start,
            duration_elements: pending.duration_elements.unwrap_or(Duration::ZERO),
            duration_render: pending.duration_render.unwrap_or(Duration::ZERO),
            duration_screencopy: pending.duration_screencopy,
            duration_displayed: pending.duration_displayed.unwrap_or(Duration::ZERO),
        }
    }
}

impl Fps {
    pub fn start(&mut self) {
        self.current_frame = Some(PendingFrame {
            start: Instant::now(),
            duration_elements: None,
            duration_render: None,
            duration_screencopy: None,
            duration_displayed: None,
        });
    }

    pub fn elements(&mut self) {
        if let Some(frame) = self.current_frame.as_mut() {
            frame.duration_elements = Some(Instant::now().duration_since(frame.start));
        }
    }

    pub fn render(&mut self) {
        if let Some(frame) = self.current_frame.as_mut() {
            frame.duration_render = Some(
                Instant::now().duration_since(frame.start)
                    - frame.duration_elements.clone().unwrap_or(Duration::ZERO),
            );
        }
    }

    pub fn screencopy(&mut self) {
        if let Some(frame) = self.current_frame.as_mut() {
            frame.duration_screencopy = Some(
                Instant::now().duration_since(frame.start)
                    - frame.duration_elements.clone().unwrap_or(Duration::ZERO)
                    - frame.duration_render.clone().unwrap_or(Duration::ZERO),
            );
        }
    }

    pub fn displayed(&mut self) {
        if let Some(mut frame) = self.current_frame.take() {
            frame.duration_displayed = Some(
                Instant::now().duration_since(frame.start)
                    - frame.duration_elements.clone().unwrap_or(Duration::ZERO)
                    - frame.duration_render.clone().unwrap_or(Duration::ZERO)
                    - frame.duration_screencopy.clone().unwrap_or(Duration::ZERO),
            );

            self.frames.push_back(frame.into());
            while self.frames.len() > 360 {
                self.frames.pop_front();
            }
        }
    }

    pub fn avg_fps(&self) -> f64 {
        if self.frames.is_empty() {
            return 0.0;
        }
        let secs = match (self.frames.front(), self.frames.back()) {
            (Some(Frame { start, .. }), Some(end_frame)) => {
                end_frame.start.duration_since(*start) + end_frame.time_to_display()
            }
            _ => Duration::ZERO,
        }
        .as_secs_f64();
        1.0 / (secs / self.frames.len() as f64)
    }

    pub fn potential_fps(&self) -> f64 {
        1.0 / self.avg_frametime().as_secs_f64()
    }

    pub fn max_frametime(&self) -> Duration {
        self.frames
            .iter()
            .map(|f| f.frame_time())
            .max()
            .unwrap_or(Duration::ZERO)
    }

    pub fn min_frametime(&self) -> Duration {
        self.frames
            .iter()
            .map(|f| f.frame_time())
            .min()
            .unwrap_or(Duration::ZERO)
    }
    pub fn avg_frametime(&self) -> Duration {
        if self.frames.is_empty() {
            return Duration::ZERO;
        }
        self.frames.iter().map(|f| f.frame_time()).sum::<Duration>() / (self.frames.len() as u32)
    }
}
