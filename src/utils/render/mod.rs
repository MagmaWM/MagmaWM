use smithay::{
    backend::renderer::{
        element::{
            surface::WaylandSurfaceRenderElement, texture::TextureRenderElement, Element, Id,
            RenderElement,
        },
        gles::element::PixelShaderElement,
        glow::GlowRenderer,
        multigpu::{gbm::GbmGlesBackend, Error as MultiError, MultiFrame, MultiRenderer},
        utils::CommitCounter,
        ImportAll, ImportMem, Renderer,
    },
    utils::{Buffer, Physical, Rectangle, Scale},
};
pub mod border;

pub type GlMultiRenderer<'a, 'b> =
    MultiRenderer<'a, 'a, 'b, GbmGlesBackend<GlowRenderer>, GbmGlesBackend<GlowRenderer>>;
pub type GlMultiFrame<'a, 'b, 'frame> =
    MultiFrame<'a, 'a, 'b, 'frame, GbmGlesBackend<GlowRenderer>, GbmGlesBackend<GlowRenderer>>;
pub enum CustomRenderElements<R>
where
    R: Renderer,
{
    Texture(TextureRenderElement<<R as Renderer>::TextureId>),
    Surface(WaylandSurfaceRenderElement<R>),
    Shader(PixelShaderElement),
}

impl<R> Element for CustomRenderElements<R>
where
    R: Renderer,
    <R as Renderer>::TextureId: 'static,
    R: ImportAll + ImportMem,
{
    fn id(&self) -> &Id {
        match self {
            CustomRenderElements::Texture(elem) => elem.id(),
            CustomRenderElements::Surface(elem) => elem.id(),
            CustomRenderElements::Shader(elem) => elem.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            CustomRenderElements::Texture(elem) => elem.current_commit(),
            CustomRenderElements::Surface(elem) => elem.current_commit(),
            CustomRenderElements::Shader(elem) => elem.current_commit(),
        }
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        match self {
            CustomRenderElements::Texture(elem) => elem.src(),
            CustomRenderElements::Surface(elem) => elem.src(),
            CustomRenderElements::Shader(elem) => elem.src(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            CustomRenderElements::Texture(elem) => elem.geometry(scale),
            CustomRenderElements::Surface(elem) => elem.geometry(scale),
            CustomRenderElements::Shader(elem) => elem.geometry(scale),
        }
    }

    fn location(&self, scale: Scale<f64>) -> smithay::utils::Point<i32, Physical> {
        match self {
            CustomRenderElements::Texture(elem) => elem.location(scale),
            CustomRenderElements::Surface(elem) => elem.location(scale),
            CustomRenderElements::Shader(elem) => elem.location(scale),
        }
    }

    fn transform(&self) -> smithay::utils::Transform {
        match self {
            CustomRenderElements::Texture(elem) => elem.transform(),
            CustomRenderElements::Surface(elem) => elem.transform(),
            CustomRenderElements::Shader(elem) => elem.transform(),
        }
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> Vec<Rectangle<i32, Physical>> {
        match self {
            CustomRenderElements::Texture(elem) => elem.damage_since(scale, commit),
            CustomRenderElements::Surface(elem) => elem.damage_since(scale, commit),
            CustomRenderElements::Shader(elem) => elem.damage_since(scale, commit),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> Vec<Rectangle<i32, Physical>> {
        match self {
            CustomRenderElements::Texture(elem) => elem.opaque_regions(scale),
            CustomRenderElements::Surface(elem) => elem.opaque_regions(scale),
            CustomRenderElements::Shader(elem) => elem.opaque_regions(scale),
        }
    }
}

impl<'a, 'b> RenderElement<GlMultiRenderer<'a, 'b>>
    for CustomRenderElements<GlMultiRenderer<'a, 'b>>
{
    fn draw<'frame>(
        &self,
        frame: &mut GlMultiFrame<'a, 'b, 'frame>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), <GlMultiRenderer<'a, 'b> as Renderer>::Error> {
        match self {
            CustomRenderElements::Texture(elem) => {
                RenderElement::<GlMultiRenderer>::draw(elem, frame, src, dst, damage)
            }
            CustomRenderElements::Surface(elem) => elem.draw(frame, src, dst, damage),
            CustomRenderElements::Shader(elem) => {
                RenderElement::<GlowRenderer>::draw(elem, frame.as_mut(), src, dst, damage)
                    .map_err(MultiError::Render)
            }
        }
    }

    fn underlying_storage(
        &self,
        renderer: &mut GlMultiRenderer<'a, 'b>,
    ) -> Option<smithay::backend::renderer::element::UnderlyingStorage> {
        match self {
            CustomRenderElements::Texture(elem) => elem.underlying_storage(renderer),
            CustomRenderElements::Surface(elem) => elem.underlying_storage(renderer),
            CustomRenderElements::Shader(elem) => elem.underlying_storage(renderer.as_mut()),
        }
    }
}

impl RenderElement<GlowRenderer> for CustomRenderElements<GlowRenderer> {
    fn draw(
        &self,
        frame: &mut <GlowRenderer as Renderer>::Frame<'_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), <GlowRenderer as Renderer>::Error> {
        match self {
            CustomRenderElements::Texture(elem) => {
                RenderElement::<GlowRenderer>::draw(elem, frame, src, dst, damage)
            }
            CustomRenderElements::Surface(elem) => elem.draw(frame, src, dst, damage),
            CustomRenderElements::Shader(elem) => {
                RenderElement::<GlowRenderer>::draw(elem, frame, src, dst, damage)
            }
        }
    }
}
impl<R> From<TextureRenderElement<<R as Renderer>::TextureId>> for CustomRenderElements<R>
where
    R: Renderer,
{
    fn from(value: TextureRenderElement<<R as Renderer>::TextureId>) -> Self {
        CustomRenderElements::Texture(value)
    }
}

impl<R> From<WaylandSurfaceRenderElement<R>> for CustomRenderElements<R>
where
    R: Renderer,
{
    fn from(value: WaylandSurfaceRenderElement<R>) -> Self {
        CustomRenderElements::Surface(value)
    }
}

impl<R> From<PixelShaderElement> for CustomRenderElements<R>
where
    R: Renderer,
{
    fn from(value: PixelShaderElement) -> Self {
        CustomRenderElements::Shader(value)
    }
}

pub trait AsGlowRenderer
where
    Self: Renderer,
{
    fn glow_renderer(&self) -> &GlowRenderer;
    fn glow_renderer_mut(&mut self) -> &mut GlowRenderer;
}

impl AsGlowRenderer for GlowRenderer {
    fn glow_renderer(&self) -> &GlowRenderer {
        self
    }

    fn glow_renderer_mut(&mut self) -> &mut GlowRenderer {
        self
    }
}

impl<'a, 'b> AsGlowRenderer for GlMultiRenderer<'a, 'b> {
    fn glow_renderer(&self) -> &GlowRenderer {
        self.as_ref()
    }

    fn glow_renderer_mut(&mut self) -> &mut GlowRenderer {
        self.as_mut()
    }
}
