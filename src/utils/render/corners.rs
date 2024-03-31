use std::borrow::BorrowMut;

use smithay::backend::{
    egl::EGLContext,
    renderer::{
        gles::{GlesRenderer, GlesTexProgram, UniformName, UniformType},
        glow::GlowRenderer,
    },
};

const CORNER_FRAG: &str = include_str!("shaders/rounded_corners.frag");

pub struct CornerShader(GlesTexProgram);

impl CornerShader {
    pub fn init(renderer: &mut GlowRenderer) {
        let renderer: &mut GlesRenderer = renderer.borrow_mut();
        let program = renderer
            .compile_custom_texture_shader(
                CORNER_FRAG,
                &[
                    UniformName::new("size", UniformType::_2f),
                    UniformName::new("radius", UniformType::_1f),
                ],
            )
            .unwrap();
        renderer
            .egl_context()
            .user_data()
            .insert_if_missing(|| CornerShader(program));
    }
    pub fn get(ctx: &EGLContext) -> GlesTexProgram {
        ctx.user_data()
            .get::<CornerShader>()
            .expect("Corner Shader not initialized")
            .0
            .clone()
    }
}
