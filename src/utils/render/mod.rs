use std::borrow::BorrowMut;

use smithay::{
    backend::{
        drm::DrmDeviceFd,
        renderer::{
            element::{
                surface::WaylandSurfaceRenderElement, texture::TextureRenderElement, Element, Id,
                RenderElement,
            },
            gles::{element::PixelShaderElement, GlesFrame, GlesTexture, Uniform},
            glow::{GlowFrame, GlowRenderer},
            multigpu::{gbm::GbmGlesBackend, Error as MultiError, MultiFrame, MultiRenderer},
            utils::{CommitCounter, DamageSet},
            ImportAll, ImportMem, Renderer, Texture,
        },
    },
    utils::{Buffer, Physical, Rectangle, Scale},
};

use crate::state::CONFIG;

use self::{border::BorderShader, corners::CornerShader};
pub mod border;
pub mod corners;

pub type GlMultiRenderer<'a> = MultiRenderer<
    'a,
    'a,
    GbmGlesBackend<GlowRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlowRenderer, DrmDeviceFd>,
>;
pub type GlMultiFrame<'a, 'frame> = MultiFrame<
    'a,
    'a,
    'frame,
    GbmGlesBackend<GlowRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlowRenderer, DrmDeviceFd>,
>;
pub enum CustomRenderElements<R>
where
    R: Renderer,
{
    Texture(TextureRenderElement<GlesTexture>),
    Surface(WaylandSurfaceRenderElement<R>),
    Shader(PixelShaderElement),
    Window(WindowRenderElement<R>),
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
            CustomRenderElements::Window(elem) => elem.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            CustomRenderElements::Texture(elem) => elem.current_commit(),
            CustomRenderElements::Surface(elem) => elem.current_commit(),
            CustomRenderElements::Shader(elem) => elem.current_commit(),
            CustomRenderElements::Window(elem) => elem.current_commit(),
        }
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        match self {
            CustomRenderElements::Texture(elem) => elem.src(),
            CustomRenderElements::Surface(elem) => elem.src(),
            CustomRenderElements::Shader(elem) => elem.src(),
            CustomRenderElements::Window(elem) => elem.src(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            CustomRenderElements::Texture(elem) => elem.geometry(scale),
            CustomRenderElements::Surface(elem) => elem.geometry(scale),
            CustomRenderElements::Shader(elem) => elem.geometry(scale),
            CustomRenderElements::Window(elem) => elem.geometry(scale),
        }
    }

    fn location(&self, scale: Scale<f64>) -> smithay::utils::Point<i32, Physical> {
        match self {
            CustomRenderElements::Texture(elem) => elem.location(scale),
            CustomRenderElements::Surface(elem) => elem.location(scale),
            CustomRenderElements::Shader(elem) => elem.location(scale),
            CustomRenderElements::Window(elem) => elem.location(scale),
        }
    }

    fn transform(&self) -> smithay::utils::Transform {
        match self {
            CustomRenderElements::Texture(elem) => elem.transform(),
            CustomRenderElements::Surface(elem) => elem.transform(),
            CustomRenderElements::Shader(elem) => elem.transform(),
            CustomRenderElements::Window(elem) => elem.transform(),
        }
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        match self {
            CustomRenderElements::Texture(elem) => elem.damage_since(scale, commit),
            CustomRenderElements::Surface(elem) => elem.damage_since(scale, commit),
            CustomRenderElements::Shader(elem) => elem.damage_since(scale, commit),
            CustomRenderElements::Window(elem) => elem.damage_since(scale, commit),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> Vec<Rectangle<i32, Physical>> {
        match self {
            CustomRenderElements::Texture(elem) => elem.opaque_regions(scale),
            CustomRenderElements::Surface(elem) => elem.opaque_regions(scale),
            CustomRenderElements::Shader(elem) => elem.opaque_regions(scale),
            CustomRenderElements::Window(elem) => elem.opaque_regions(scale),
        }
    }
}

impl<'a> RenderElement<GlMultiRenderer<'a>> for CustomRenderElements<GlMultiRenderer<'a>> {
    fn draw<'frame>(
        &self,
        frame: &mut GlMultiFrame<'a, 'frame>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), <GlMultiRenderer<'a> as Renderer>::Error> {
        match self {
            CustomRenderElements::Texture(elem) => {
                RenderElement::<GlowRenderer>::draw(elem, frame.as_mut(), src, dst, damage)
                    .map_err(MultiError::Render)
            }
            CustomRenderElements::Surface(elem) => elem.draw(frame, src, dst, damage),
            CustomRenderElements::Shader(elem) => {
                RenderElement::<GlowRenderer>::draw(elem, frame.as_mut(), src, dst, damage)
                    .map_err(MultiError::Render)
            }
            CustomRenderElements::Window(elem) => elem.draw(frame, src, dst, damage),
        }
    }

    fn underlying_storage(
        &self,
        renderer: &mut GlMultiRenderer<'a>,
    ) -> Option<smithay::backend::renderer::element::UnderlyingStorage> {
        match self {
            CustomRenderElements::Texture(elem) => elem.underlying_storage(renderer.as_mut()),
            CustomRenderElements::Surface(elem) => elem.underlying_storage(renderer),
            CustomRenderElements::Shader(elem) => elem.underlying_storage(renderer.as_mut()),
            CustomRenderElements::Window(elem) => elem.underlying_storage(renderer),
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
            CustomRenderElements::Window(elem) => elem.draw(frame, src, dst, damage),
        }
    }
}
impl<R> From<TextureRenderElement<GlesTexture>> for CustomRenderElements<R>
where
    R: Renderer + AsGlowRenderer,
{
    fn from(value: TextureRenderElement<GlesTexture>) -> Self {
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

impl<R> From<WindowRenderElement<R>> for CustomRenderElements<R>
where
    R: Renderer,
{
    fn from(value: WindowRenderElement<R>) -> Self {
        CustomRenderElements::Window(value)
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

impl<'a> AsGlowRenderer for GlMultiRenderer<'a> {
    fn glow_renderer(&self) -> &GlowRenderer {
        self.as_ref()
    }

    fn glow_renderer_mut(&mut self) -> &mut GlowRenderer {
        self.as_mut()
    }
}

pub struct WindowRenderElement<R>
where
    R: Renderer,
{
    inner: WaylandSurfaceRenderElement<R>,
}

impl<R> Element for WindowRenderElement<R>
where
    R: Renderer,
    <R as Renderer>::TextureId: 'static,
    R: ImportAll + ImportMem,
{
    fn id(&self) -> &Id {
        self.inner.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.inner.src()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }

    fn location(&self, scale: Scale<f64>) -> smithay::utils::Point<i32, Physical> {
        self.inner.location(scale)
    }

    fn transform(&self) -> smithay::utils::Transform {
        self.inner.transform()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, smithay::utils::Physical> {
        self.inner.damage_since(scale, commit)
    }

    fn opaque_regions(&self, _scale: Scale<f64>) -> Vec<Rectangle<i32, Physical>> {
        // PERF: actually compute opaque regions
        vec![]
    }

    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }

    fn kind(&self) -> smithay::backend::renderer::element::Kind {
        self.inner.kind()
    }
}

impl<'a> RenderElement<GlMultiRenderer<'a>> for WindowRenderElement<GlMultiRenderer<'a>> {
    fn draw(
        &self,
        frame: &mut <GlMultiRenderer<'a> as Renderer>::Frame<'_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), <GlMultiRenderer<'a> as Renderer>::Error> {
        // apply shader to round corners (tty)
        if CONFIG.borders.radius > 0.0 {
            let size = self
                .geometry(Scale::from(1.0))
                .size
                .to_logical(Scale::from(1));
            let framegl = <GlowFrame<'_> as BorrowMut<GlesFrame>>::borrow_mut(frame.as_mut());
            framegl.override_default_tex_program(
                CornerShader::get(framegl.egl_context()),
                vec![
                    Uniform::new("size", [size.w as f32, size.h as f32]),
                    Uniform::new("radius", CONFIG.borders.radius),
                ],
            );
            self.inner.draw(frame, src, dst, damage)?;
            <GlowFrame<'_> as BorrowMut<GlesFrame>>::borrow_mut(frame.as_mut())
                .clear_tex_program_override();
        } else {
            self.inner.draw(frame, src, dst, damage)?;
        }

        Ok(())
    }

    fn underlying_storage(
        &self,
        renderer: &mut GlMultiRenderer<'a>,
    ) -> Option<smithay::backend::renderer::element::UnderlyingStorage> {
        self.inner.underlying_storage(renderer)
    }
}

impl RenderElement<GlowRenderer> for WindowRenderElement<GlowRenderer> {
    fn draw(
        &self,
        frame: &mut <GlowRenderer as Renderer>::Frame<'_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), <GlowRenderer as Renderer>::Error> {
        // apply shader to round corners (winit)
        if CONFIG.borders.radius > 0.0 {
            let size = self
                .geometry(Scale::from(1.0))
                .size
                .to_logical(Scale::from(1));
            let framegl = <GlowFrame<'_> as BorrowMut<GlesFrame>>::borrow_mut(frame);
            framegl.override_default_tex_program(
                CornerShader::get(framegl.egl_context()),
                vec![
                    Uniform::new("size", [size.w as f32, size.h as f32]),
                    Uniform::new("radius", CONFIG.borders.radius),
                ],
            );
            self.inner.draw(frame, src, dst, damage)?;
            <GlowFrame<'_> as BorrowMut<GlesFrame>>::borrow_mut(frame).clear_tex_program_override();
        } else {
            self.inner.draw(frame, src, dst, damage)?;
        }
        Ok(())
    }

    fn underlying_storage(
        &self,
        renderer: &mut GlowRenderer,
    ) -> Option<smithay::backend::renderer::element::UnderlyingStorage> {
        self.inner.underlying_storage(renderer)
    }
}

impl<R> From<WaylandSurfaceRenderElement<R>> for WindowRenderElement<R>
where
    R: Renderer,
{
    fn from(value: WaylandSurfaceRenderElement<R>) -> Self {
        WindowRenderElement { inner: value }
    }
}

// wraps the parent surface of a window in a window element for rendering
pub fn wrap_window_surface<
    R: Renderer + ImportAll + AsGlowRenderer,
    C: From<WaylandSurfaceRenderElement<R>> + From<WindowRenderElement<R>>,
>(
    mut elements: Vec<WaylandSurfaceRenderElement<R>>,
) -> Vec<C>
where
    <R as Renderer>::TextureId: Texture + 'static,
{
    if let Some(elem) = elements.pop() {
        let win = WindowRenderElement::from(elem);
        let mut elements: Vec<C> = elements.into_iter().map(C::from).collect();
        elements.push(C::from(win));
        elements
    } else {
        elements.into_iter().map(C::from).collect()
    }
}

pub fn init_shaders(renderer: &mut GlowRenderer) {
    BorderShader::init(renderer);
    CornerShader::init(renderer);
}
