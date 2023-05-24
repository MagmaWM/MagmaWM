use smithay::{
    backend::renderer::gles::{
        element::PixelShaderElement, GlesPixelProgram, GlesRenderer, Uniform, UniformName,
        UniformType,
    },
    utils::{Logical, Point, Rectangle, Size},
};

use crate::state::CONFIG;

const ROUNDED_BORDER_FRAG: &str = include_str!("shaders/rounded_borders.frag");
const BORDER_FRAG: &str = include_str!("shaders/borders.frag");
pub struct BorderShader {
    rounded: GlesPixelProgram,
    default: GlesPixelProgram,
}

impl BorderShader {
    pub fn init(renderer: &mut GlesRenderer) {
        let rounded = renderer
            .compile_custom_pixel_shader(
                ROUNDED_BORDER_FRAG,
                &[
                    UniformName::new("startColor", UniformType::_3f),
                    UniformName::new("endColor", UniformType::_3f),
                    UniformName::new("thickness", UniformType::_1f),
                    UniformName::new("radius", UniformType::_1f),
                    UniformName::new("angle", UniformType::_1f),
                ],
            )
            .unwrap();
        let default = renderer
            .compile_custom_pixel_shader(
                BORDER_FRAG,
                &[
                    UniformName::new("startColor", UniformType::_3f),
                    UniformName::new("endColor", UniformType::_3f),
                    UniformName::new("thickness", UniformType::_1f),
                    UniformName::new("angle", UniformType::_1f),
                ],
            )
            .unwrap();
        renderer
            .egl_context()
            .user_data()
            .insert_if_missing(|| BorderShader { rounded, default });
    }
    pub fn get(renderer: &mut GlesRenderer) -> &BorderShader {
        renderer
            .egl_context()
            .user_data()
            .get::<BorderShader>()
            .expect("Border Shader not initialized")
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
        if CONFIG.borders.radius > 0.0 {
            PixelShaderElement::new(
                Self::get(renderer).rounded.clone(),
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
                    Uniform::new("radius", CONFIG.borders.radius + thickness + 2.0),
                    Uniform::new("angle", CONFIG.borders.gradient_angle),
                ],
            )
        } else {
            PixelShaderElement::new(
                Self::get(renderer).default.clone(),
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
                    Uniform::new("angle", CONFIG.borders.gradient_angle),
                ],
            )
        }
    }
}