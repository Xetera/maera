use ratmom::{http::Method, AsyncBody};
use std::{
    marker::{Send, Sync},
    time::Duration,
};

use crate::MaeraResponse;

/// Declarative chaining of requests
// #[derive(Clone)]
pub enum Chain<K> {
    /// Final product
    End(K),
    Next(
        ChainableRequest,
        Box<dyn Fn(MaeraResponse) -> Chain<K> + Sync + Send + 'static>,
    ),
}

impl<K> Chain<K> {
    pub fn end(k: K) -> Self {
        Chain::End(k)
    }
    pub fn next<F>(request: ChainableRequest, f: F) -> Self
    where
        F: Fn(MaeraResponse) -> Chain<K> + Sync + Send + 'static,
    {
        Chain::Next(request, Box::new(f))
    }
    /// Executes the chain of requests
    pub(crate) async fn run(self, client: &ratmom::HttpClient) -> Result<K, ratmom::Error> {
        let mut next = self;
        let mut first_run = true;
        loop {
            match next {
                Chain::End(k) => return Ok(k),
                Chain::Next(chainable, f) => {
                    // We wanna make sure that we don't sleep on the first run
                    if !first_run {
                        tokio::time::sleep(chainable.delay).await;
                        first_run = false;
                    }
                    let request: ratmom::http::Request<AsyncBody> = chainable.into();
                    let result = client.send_async(request).await?;
                    next = f(result);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ChainableRequest {
    pub url: String,
    pub method: Method,
    /// Headers are all appended
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub delay: Duration,
}

impl From<ChainableRequestBuilder> for Chain<MaeraResponse> {
    fn from(k: ChainableRequestBuilder) -> Self {
        Chain::next(k.build(), Chain::end)
    }
}

impl From<ChainableRequest> for Chain<MaeraResponse> {
    fn from(k: ChainableRequest) -> Self {
        Chain::next(k, Chain::end)
    }
}
/// Helper conversion for the auth method
// impl From<ChainableRequest> for Chain<Vec<Cookie>> {
//     fn from(k: ChainableRequest) -> Self {
//         Chain::one(k)
//     }
// }

#[derive(Default)]
pub struct ChainableRequestBuilder {
    base_url: Option<String>,
    url: Option<String>,
    method: Option<Method>,
    /// Headers are all appended
    headers: Vec<(String, String)>,
    body: Option<String>,
    delay: Option<Duration>,
}

impl ChainableRequestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: Some(base_url.into()),
            ..Default::default()
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(format!(
            "{}{}",
            self.base_url.clone().unwrap_or_default(),
            url.into()
        ));
        self
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    pub fn header(mut self, key: String, value: String) -> Self {
        self.headers.push((key, value));
        self
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn build(self) -> ChainableRequest {
        ChainableRequest {
            url: self.url.unwrap(),
            method: self.method.unwrap_or(Method::GET),
            headers: self.headers,
            body: self.body,
            delay: self.delay.unwrap_or_default(),
        }
    }
}

impl From<ChainableRequest> for ratmom::http::Request<AsyncBody> {
    fn from(req: ChainableRequest) -> Self {
        let mut builder = ratmom::http::Request::builder()
            .method(req.method)
            .uri(req.url);

        for (key, value) in req.headers {
            builder = builder.header(key, value);
        }

        if let Some(body) = req.body {
            builder.body(body.into()).unwrap()
        } else {
            builder.body(AsyncBody::empty()).unwrap()
        }
    }
}
