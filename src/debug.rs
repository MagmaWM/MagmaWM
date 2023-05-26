use std::{cell::RefCell, rc::Rc};

use egui::{Context, FullOutput};
use egui_glow::Painter;
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::texture::{TextureRenderBuffer, TextureRenderElement},
            gles::{GlesError, GlesTexture},
            glow::GlowRenderer,
            Bind, Frame, Offscreen, Renderer, Unbind,
        },
    },
    utils::{Logical, Rectangle, Transform},
};

struct GlState {
    painter: Painter,
    render_buffer: TextureRenderBuffer<GlesTexture>,
}
type UserDataType = Rc<RefCell<GlState>>;

#[derive(Default)]
pub struct MagmaEgui {
    ctx: Context,
    pub active: bool,
}

impl MagmaEgui {
    pub fn render(
        &mut self,
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
        } = self.ctx.run(input, MagmaEgui::ui);

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

    fn ui(ctx: &Context) {
        egui::Area::new("main")
            .anchor(egui::Align2::LEFT_TOP, (10.0, 10.0))
            .show(ctx, |ui| {
                ui.label(format!(
                    "MagmaWM version {}",
                    std::env!("CARGO_PKG_VERSION")
                ));
                if let Some(hash) = std::option_env!("GIT_HASH").and_then(|x| x.get(0..10)) {
                    ui.label(format!("git: {hash}"));
                }
            });
    }
}
