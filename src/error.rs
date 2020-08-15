use std::error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use fchat3_log_lib::error as lib_error;
use handlebars::RenderError;
use std::io;

#[derive(Debug)]
pub enum Error {
    LibError(lib_error::Error),
    RenderError(RenderError),
    IOError(io::Error)
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "failed to write or read message"
    }
}

impl From<lib_error::Error> for Error {
    fn from(item: lib_error::Error) -> Self {
        Self::LibError(item)
    }
}

impl From<RenderError> for Error {
    fn from(item: RenderError) -> Self {
        Self::RenderError(item)
    }
}

impl From<io::Error> for Error {
    fn from(item: io::Error) -> Self {
        Self::IOError(item)
    }
}