use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("window creation failed: {0}")]
    Create(String),
    #[error("run failed: {0}")]
    Run(String),
}
