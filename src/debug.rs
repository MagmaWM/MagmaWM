use std::{cell::RefCell, rc::Rc};

use egui::{Context, FullOutput};
use egui_glow::Painter;
use smithay::{
    backend::{
        allocator::Fourcc,
        drm::DrmNode,
        renderer::{
            element::texture::{TextureRenderBuffer, TextureRenderElement},
            gles::{GlesError, GlesTexture},
            glow::GlowRenderer,
            Bind, Frame, Offscreen, Renderer, Unbind,
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
