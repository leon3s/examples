use std::task::{Context, Poll};

use ntex::http::body::{Body, BodySize, MessageBody, ResponseBody};
use ntex::service::{Middleware, Service};
use ntex::util::{BoxFuture, Bytes, BytesMut};
use ntex::web::{Error, ErrorRenderer, WebRequest, WebResponse};

pub struct Logging;

impl<S> Middleware<S> for Logging {
    type Service = LoggingMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        LoggingMiddleware { service }
    }
}

pub struct LoggingMiddleware<S> {
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for LoggingMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
    Err: ErrorRenderer,
{
    type Response = WebResponse;
    type Error = Error;
    type Future<'f> = BoxFuture<'f, Result<Self::Response, Self::Error>> where Self: 'f;

    ntex::forward_poll_ready!(service);

    fn call(&self, req: WebRequest<Err>) -> Self::Future<'_> {
        Box::pin(async move {
            self.service.call(req).await.map(|res| {
                res.map_body(move |_, body| {
                    Body::from_message(BodyLogger {
                        body,
                        body_accum: BytesMut::new(),
                    })
                    .into()
                })
            })
        })
    }
}

pub struct BodyLogger {
    body: ResponseBody<Body>,
    body_accum: BytesMut,
}

impl Drop for BodyLogger {
    fn drop(&mut self) {
        println!("response body: {:?}", self.body_accum);
    }
}

impl MessageBody for BodyLogger {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next_chunk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Bytes, Box<dyn std::error::Error>>>> {
        match self.body.poll_next_chunk(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.body_accum.extend_from_slice(&chunk);
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
