use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

pub use errbit::Config;
use errbit::Notifier;

use actix_service::{Service, Transform};
use actix_utils::future::{ok, Ready};
use actix_web::error::Error as WebError;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse};

mod error;
pub use error::{ErrbitError, Error};

/// Reports errors to errbit.
#[derive(Debug)]
pub struct Errbit(Rc<Inner>);

#[derive(Debug, Clone)]
struct Inner {
    notifier: Notifier,
}

impl Errbit {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        Ok(Self(Rc::new(Inner {
            notifier: Notifier::new(config)?,
        })))
    }
}

impl Default for Errbit {
    fn default() -> Self {
        let config = Config::default();
        Self::new(config).expect("the errbit endpoint configuration is not valid...")
    }
}

impl<S> Transform<S, ServiceRequest> for Errbit
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = WebError>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = WebError;
    type InitError = ();
    type Transform = ErrbitMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ErrbitMiddleware {
            inner: self.0.clone(),
            service,
        })
    }
}

pub struct ErrbitMiddleware<S> {
    inner: Rc<Inner>,
    service: S,
}

impl<S, B> Service<ServiceRequest> for ErrbitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = WebError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = WebError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let inner = self.inner.clone();
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = match fut.await {
                Ok(res) => {
                    if res.response().status().is_server_error() {
                        if let Some(err) = res.response().error() {
                            match err.as_error::<Error>() {
                                Some(e) => match e.as_anyhow_error() {
                                    Some(anyhow_error) => {
                                        if let Err(notify_error) =
                                            inner.notifier.notify_anyhow_error(anyhow_error).await
                                        {
                                            println!("{}", notify_error)
                                        }
                                    }
                                    None => {
                                        if let Err(notify_error) =
                                            inner.notifier.notify_error(e).await
                                        {
                                            println!("{}", notify_error)
                                        }
                                    }
                                },
                                None => {
                                    if let Err(notify_error) =
                                        inner.notifier.notify_error(&err).await
                                    {
                                        println!("{}", notify_error)
                                    }
                                }
                            };
                        }
                    }
                    Ok(res)
                }
                Err(err) => {
                    match err.as_error::<Error>() {
                        Some(e) => match e.as_anyhow_error() {
                            Some(anyhow_error) => {
                                if let Err(notify_error) =
                                    inner.notifier.notify_anyhow_error(anyhow_error).await
                                {
                                    println!("{}", notify_error)
                                }
                            }
                            None => {
                                if let Err(notify_error) = inner.notifier.notify_error(e).await {
                                    println!("{}", notify_error)
                                }
                            }
                        },
                        None => {
                            if let Err(notify_error) = inner.notifier.notify_error(&err).await {
                                println!("{}", notify_error)
                            }
                        }
                    };
                    Err(err)
                }
            }?;
            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrbitError;
    use actix_service::IntoService;
    use actix_utils::future::ok;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::{HttpResponse, ResponseError};
    use anyhow::Context;

    #[derive(Clone, Debug)]
    pub struct MyTestError(String);

    impl std::fmt::Display for MyTestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl ResponseError for MyTestError {}

    pub struct MyTestAnyhowError(anyhow::Error);

    impl std::fmt::Debug for MyTestAnyhowError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl std::fmt::Display for MyTestAnyhowError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl ResponseError for MyTestAnyhowError {}

    impl ErrbitError for MyTestAnyhowError {
        fn as_anyhow(&self) -> Option<&anyhow::Error> {
            Some(&self.0)
        }
    }

    #[actix_rt::test]
    #[serial_test::serial]
    async fn test_response_ok() {
        dotenv::dotenv().ok();
        let srv = |req: ServiceRequest| {
            ok(req.into_response(
                HttpResponse::build(StatusCode::OK)
                    .insert_header(("X-Test", "ttt"))
                    .finish(),
            ))
        };
        let config = Config::default();
        let errbit = Errbit::new(config).expect("must be configured...");
        let srv = errbit.new_transform(srv.into_service()).await.unwrap();
        let req = TestRequest::default().to_srv_request();
        let _res = srv.call(req).await;
    }

    #[actix_rt::test]
    #[serial_test::serial]
    async fn test_response_std_error() {
        dotenv::dotenv().ok();
        let srv = |req: ServiceRequest| {
            let double_number =
                |number_str: &str| -> std::result::Result<i32, std::num::ParseIntError> {
                    number_str.parse::<i32>().map(|n| 2 * n)
                };
            let parse_error = double_number("NOT A NUMBER").err().unwrap();
            ok(req.error_response(MyTestError(format!("{}", parse_error))))
        };
        let config = Config::default();
        let errbit = Errbit::new(config).expect("must be configured...");
        let srv = errbit.new_transform(srv.into_service()).await.unwrap();
        let req = TestRequest::default().to_srv_request();
        let _res = srv.call(req).await;
    }

    #[actix_rt::test]
    #[serial_test::serial]
    async fn test_response_anyhow_error() {
        dotenv::dotenv().ok();
        let srv = |req: ServiceRequest| {
            let double_number = |number_str: &str| -> anyhow::Result<i32> {
                number_str
                    .parse::<i32>()
                    .map(|n| 2 * n)
                    .with_context(|| format!("Failed to parse number_str of {}", number_str))
            };
            let parse_error = double_number("NOT A NUMBER").err().unwrap();
            ok(req.error_response(Error::from(MyTestAnyhowError(parse_error))))
        };
        let config = Config::default();
        let errbit = Errbit::new(config).expect("must be configured...");
        let srv = errbit.new_transform(srv.into_service()).await.unwrap();
        let req = TestRequest::default().to_srv_request();
        let _res = srv.call(req).await;
    }
}
