use smithay::backend::{renderer::gles::GlesError, SwapBuffersError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Shader compilation error: {0}")]
    ShaderCompilationError(#[from] GlesError),
    #[error("Border shader not initialized")]
    BorderShaderNotInitialized,
    #[error("Swap buffers error: {0}")]
    SwapBuffersError(#[from] SwapBuffersError),
}

pub type Result<T> = std::result::Result<T, Error>;
