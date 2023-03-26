use std::{
    str::FromStr,
    sync::{atomic::AtomicU32, Arc},
};

use async_trait::async_trait;
use ratmom::{
    cookies::{Cookie, CookieJar},
    http::Uri,
    AsyncBody, HttpClient, Request, Response,
};

use crate::{request::Chain, ChainableRequestBuilder};

// use crate::auth::JobAuthorizer;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36";

pub type MaeraRequest = Request<AsyncBody>;
pub type MaeraResponse = Response<AsyncBody>;
pub type MaeraError = ratmom::Error;

/// The decision process after a request is made
pub enum Decision {
    /// Do nothing, continue scraping as usual
    Continue,
    /// Runs the authorization process immediately
    Authorize,
    /// Stops the monitor from running
    Stop,
}

pub type AuthorizeNext = Chain<Vec<Cookie>>;

// type Authorizer = Box<AuthorizeNext>;

#[async_trait]
pub trait JobHandler: Send + Sync + 'static {
    type Response: Send;
    // fn authorize(&self) -> AuthorizeNext {
    //     // no cookies passed up by default
    //     Chain::End(vec![])
    // }

    fn request(&self, builder: ChainableRequestBuilder) -> Chain<Self::Response>;
    /// Called when a request is successfully made
    async fn on_success(&self, response: &mut Self::Response) -> Decision;
    /// Called when a request fails
    async fn on_error(&self, _error: MaeraError) -> Decision {
        Decision::Continue
    }
}

pub type Authorizer = Box<dyn Fn() -> AuthorizeNext + Send + Sync + 'static>;
// pub trait Authorizer: Send + Sync + 'static {
//     fn authorize(&self) -> AuthorizeNext;
// }

pub struct Job<T: JobHandler> {
    /// The name of the job
    // TODO: allow for multiple handlers under a single job
    pub handler: Arc<T>,
    pub authorizer: Option<Authorizer>,
    pub base_url: String,
    cookie_jar: CookieJar,
    #[allow(dead_code)]
    auth_retries: AtomicU32,
}

impl<T: JobHandler> From<JobBuilder<T>> for Job<T> {
    fn from(value: JobBuilder<T>) -> Job<T> {
        Job {
            cookie_jar: value.cookie_jar.unwrap_or_default(),
            base_url: value.base_url.expect("Missing base_url"),
            handler: Arc::new(value.handler.expect("Missing handler")),
            authorizer: value.authorizer,
            auth_retries: AtomicU32::default(),
        }
    }
}

pub struct JobBuilder<T: JobHandler> {
    cookie_jar: Option<CookieJar>,
    base_url: Option<String>,
    handler: Option<T>,
    authorizer: Option<Authorizer>,
}

impl<T: JobHandler> Default for JobBuilder<T> {
    fn default() -> Self {
        Self {
            cookie_jar: None,
            base_url: None,
            handler: None,
            authorizer: None,
        }
    }
}

impl<T: JobHandler> JobBuilder<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cookie_jar(mut self, cookie_jar: CookieJar) -> Self {
        self.cookie_jar = Some(cookie_jar);
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn handler(mut self, handler: T) -> Self {
        self.handler = Some(handler);
        self
    }

    pub fn authorizer<F>(mut self, authorizer: F) -> Self
    where
        F: Fn() -> AuthorizeNext + Send + Sync + 'static,
    {
        self.authorizer = Some(Box::new(authorizer));
        self
    }

    pub fn build(self) -> Job<T> {
        self.into()
    }
}

// pub struct RequestOptions {
// }
// impl Default for RequestOptions {
//     fn default() -> Self {
//         Self { method: Method::GET, append_headers: vec![] }
//     }
// }

// struct Domain {
//     url
// }

// #[derive(Clone)]
// #[derive(Send)]
pub struct Maera<T: JobHandler> {
    client: Arc<ratmom::HttpClient>,
    pub jobs: Vec<Job<T>>,
    // pub running: Vec<u8>,
}

impl<T: JobHandler> Maera<T> {
    pub fn new(jobs: Vec<Job<T>>) -> Self {
        let client = Arc::new(ratmom::HttpClientBuilder::new().default_headers(&[
            ("cache-control", "no-cache"),
            ("sec-ch-ua", "\"Google Chrome\";v=\"111\", \"Not(A:Brand\";v=\"8\", \"Chromium\";v=\"111\""),
            ("sec-ch-ua-mobile", "?0"),
            ("sec-ch-ua-platform", "Windows"),
            // ("sec-ch-ua-platform", "macOS"),
            // this seems to mess with the order of the headers
            // ("dnt", "1"),
            ("Upgrade-Insecure-Requests", "1"),
            ("User-Agent", DEFAULT_USER_AGENT),
            // ("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36"),
            ("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"),
            ("Sec-Fetch-Site", "none"),
            ("Sec-Fetch-Mode", "navigate"),
            ("Sec-Fetch-User", "?1"),
            ("Sec-Fetch-Dest", "document"),
            ("Accept-Encoding", "gzip, deflate, br"),
            ("Accept-Language", "en-US,en;q=0.9"),
        ]).build().unwrap());
        Self { jobs, client }
    }

    /// Authorization function to return cookies
    async fn authorize(client: &HttpClient, job: &Arc<Job<T>>) -> Result<(), ratmom::Error>
    where
        T: JobHandler,
    {
        if let Some(ref authorize) = &job.authorizer {
            let cookie_chain = authorize();
            let result = cookie_chain.run(client).await?;
            // We can just use the base url for the cookie here that should be
            let uri = Uri::from_str(&job.base_url).expect("invalid base url");
            for cookie in result.into_iter() {
                // TODO: error handling hours
                job.cookie_jar.set(cookie, &uri).unwrap();
            }
        }
        Ok(())
    }

    async fn send_request(
        client: &Arc<HttpClient>,
        chain: Chain<T::Response>,
        handler: &Arc<T>,
    ) -> Decision
    where
        T: JobHandler,
    {
        match chain.run(client).await {
            Ok(mut response) => handler.on_success(&mut response).await,
            Err(err) => handler.on_error(err).await,
        }
    }

    async fn run_job(client: &Arc<HttpClient>, job: &Arc<Job<T>>)
    where
        T: JobHandler,
    {
        let builder = ChainableRequestBuilder::from_base_url(job.base_url.clone());
        let chain = job.handler.request(builder);
        Maera::send_request(client, chain, &Arc::clone(&job.handler)).await;
        // match decision {
        //     Decision::Authorize => {
        //         Maera::authorize(&client, &job).await.unwrap();
        //         Maera::run_job(client, job).await;
        //     }
        //     Decision::Stop => {
        //         todo!("Stopping is not implemented yet")
        //     }
        //     Decision::Continue => {}
        // }
    }

    pub async fn start(self) -> Result<(), tokio::task::JoinError> {
        let Maera { jobs, client } = self;

        for job in jobs.into_iter() {
            let job = Arc::new(job);

            tokio::spawn({
                let job = Arc::clone(&job);
                let client = Arc::clone(&client);
                async move {
                    Maera::authorize(&client, &job).await.unwrap();
                    // if let Some(authorize) = &job.authorizer {
                    //     authorize();
                    // }
                    loop {
                        Maera::run_job(&client, &job).await;
                    }
                }
            });
        }
        Ok(())
    }
}
