use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, texture::TextureRenderElement},
        ImportAll, ImportMem, Renderer,
    },
    render_elements,
};

render_elements! {
    pub CustomRenderElements<R> where
        R: ImportAll + ImportMem;
    Texture=TextureRenderElement<<R as Renderer>::TextureId>,
    Surface=WaylandSurfaceRenderElement<R>,
}
