use smithay::{
    backend::renderer::gles::{
        element::PixelShaderElement, GlesPixelProgram, GlesRenderer, Uniform, UniformName,
        UniformType,
    },
    utils::{Logical, Point, Rectangle, Size},
};

use crate::state::CONFIG;

const BORDER_FRAG: &str = include_str!("shaders/borders.frag");
pub struct BorderShader(pub GlesPixelProgram);

impl BorderShader {
    pub fn init(renderer: &mut GlesRenderer) {
        let shader = renderer
            .compile_custom_pixel_shader(
                BORDER_FRAG,
                &[
                    UniformName::new("startColor", UniformType::_3f),
                    UniformName::new("endColor", UniformType::_3f),
                    UniformName::new("thickness", UniformType::_1f),
                    UniformName::new("radius", UniformType::_1f),
                    UniformName::new("angle", UniformType::_1f),
                ],
            )
            .unwrap();
        renderer
            .egl_context()
            .user_data()
            .insert_if_missing(|| BorderShader(shader));
    }
    pub fn get(renderer: &mut GlesRenderer) -> GlesPixelProgram {
        renderer
            .egl_context()
            .user_data()
            .get::<BorderShader>()
            .expect("Border Shader not initialized")
            .0
            .clone()
    }
    pub fn element(
        renderer: &mut GlesRenderer,
        geo: Rectangle<i32, Logical>,
    ) -> PixelShaderElement {
        let thickness: f32 = CONFIG.borders.thickness as f32;
        let thickness_loc = (thickness as i32, thickness as i32);
        let thickness_size = ((thickness * 2.0) as i32, (thickness * 2.0) as i32);
        let geo = Rectangle::from_loc_and_size(
            geo.loc - Point::from(thickness_loc),
            geo.size + Size::from(thickness_size),
        );
        PixelShaderElement::new(
            Self::get(renderer),
            geo,
            None,
            1.0,
            vec![
                Uniform::new("startColor", CONFIG.borders.start_color),
                Uniform::new(
                    "endColor",
                    CONFIG
                        .borders
                        .end_color
                        .unwrap_or(CONFIG.borders.start_color),
                ),
                Uniform::new("thickness", thickness),
                Uniform::new("radius", CONFIG.borders.radius),
                Uniform::new("angle", CONFIG.borders.gradient_angle),
            ],
        )
    }
}
