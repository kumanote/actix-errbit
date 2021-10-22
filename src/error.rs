use actix_web::{HttpResponse, ResponseError};

pub struct Error {
    cause: Box<dyn ErrbitError>,
}

impl<T: ErrbitError + 'static> From<T> for Error {
    fn from(err: T) -> Error {
        Error {
            cause: Box::new(err),
        }
    }
}

impl Error {
    pub fn as_anyhow_error(&self) -> Option<&anyhow::Error> {
        self.cause.as_anyhow()
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.cause, f)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self.cause)
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        self.cause.error_response()
    }
}

pub trait ErrbitError: ResponseError {
    fn as_anyhow(&self) -> Option<&anyhow::Error>;
}
