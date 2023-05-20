use smithay::{
    backend::renderer::gles::{
        element::PixelShaderElement, GlesRenderer, Uniform, UniformName, UniformType,
    },
    utils::{Logical, Point, Rectangle, Size},
};

const BORDER_FRAG: &str = include_str!("shaders/borders.frag");
pub struct BorderShader;

impl BorderShader {
    pub fn element(
        renderer: &mut GlesRenderer,
        geo: Rectangle<i32, Logical>,
    ) -> PixelShaderElement {
        let thickness: f32 = 8.0;
        let thickness_loc = (thickness as i32, thickness as i32);
        let thickness_size = ((thickness * 2.0) as i32, (thickness * 2.0) as i32);
        let geo = Rectangle::from_loc_and_size(
            geo.loc - Point::from(thickness_loc),
            geo.size + Size::from(thickness_size),
        );
        let shader = renderer
            .compile_custom_pixel_shader(
                BORDER_FRAG,
                &[
                    UniformName::new("color", UniformType::_3f),
                    UniformName::new("thickness", UniformType::_1f),
                    UniformName::new("radius", UniformType::_1f),
                ],
            )
            .unwrap();
        PixelShaderElement::new(
            shader,
            geo,
            None,
            1.0,
            vec![
                Uniform::new("color", [0.580, 0.921, 0.921]),
                Uniform::new("thickness", thickness),
                Uniform::new("radius", thickness * 2.0),
            ],
        )
    }
}
