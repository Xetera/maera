use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use clokwerk::{AsyncJob, AsyncScheduler};
use ratmom::{AsyncBody, Request, Response};

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36";

pub type MaeraRequest = Request<AsyncBody>;
pub type MaeraResponse = Response<AsyncBody>;
pub type MaeraError = ratmom::Error;

#[async_trait]
pub trait JobHandler: Send + Clone + Sync + 'static {
    /// The website that's going to be monitored
    fn target(&self) -> MaeraRequest;
    /// Called when a request is successfully made
    async fn on_success(&self, response: &mut MaeraResponse);
    /// Called when a request fails
    async fn on_error(&self, error: MaeraError);
    /// The schedule for how often the site should be scraped
    fn schedule<'a>(&self, scheduler: &'a mut AsyncScheduler) -> &'a mut AsyncJob;
}

#[derive(Clone)]
pub struct Job<T: JobHandler> {
    /// The name of the job
    pub name: String,
    pub handler: T,
}

#[derive(Clone)]
pub struct Maera<T: JobHandler> {
    client: Arc<ratmom::HttpClient>,
    pub jobs: Vec<Job<T>>,
}

impl<T: JobHandler> Maera<T> {
    pub fn new(jobs: Vec<Job<T>>) -> Self {
        let client = Arc::new(ratmom::HttpClientBuilder::new().default_headers(&[
            ("cache-control", "no-cache"),
            ("sec-ch-ua", "\"Google Chrome\";v=\"111\", \"Not(A:Brand\";v=\"8\", \"Chromium\";v=\"111\""),
            ("sec-ch-ua-mobile", "?0"),
            ("sec-ch-ua-platform", "Windows"),
            // this seems to mess with the order of the headers
            // ("dnt", "1"),
            ("Upgrade-Insecure-Requests", "1"),
            ("User-Agent", DEFAULT_USER_AGENT),
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

    async fn run(&self, req: Request<AsyncBody>, handler: Arc<T>)
    where
        T: JobHandler,
    {
        let response_future = self.client.send_async(req);
        match response_future.await {
            Ok(mut response) => {
                handler.on_success(&mut response).await;
            }
            Err(err) => {
                handler.on_error(err).await;
            }
        }
    }

    pub async fn start(self) -> Result<(), tokio::task::JoinError> {
        let mut scheduler = AsyncScheduler::new();
        let jobs = self.jobs.clone();
        let arc_self = Arc::new(self);

        for job in jobs.into_iter() {
            let clone_handle = job.handler.clone();
            let task = clone_handle.schedule(&mut scheduler);
            let handler = Arc::new(job.handler);
            let arc_self = Arc::clone(&arc_self);
            task.run(move || {
                let req = handler.target();
                let handler = Arc::clone(&handler);
                let arc_self = arc_self.clone();
                async move {
                    arc_self.run(req, handler.clone()).await;
                }
            });
        }
        tokio::spawn(async move {
            loop {
                scheduler.run_pending().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use clokwerk::Job;
//     use ratmom::http::Method;

//     #[derive(Clone)]
//     struct TestHandler {
//         pub name: String,
//     }

//     #[async_trait]
//     impl JobHandler for TestHandler {
//         fn target(&self) -> MaeraRequest {
//             Request::builder()
//                 .method(Method::GET)
//                 .uri("https://www.google.com")
//                 .body(AsyncBody::empty())
//                 .unwrap()
//         }

//         async fn on_success(&self, response: &mut MaeraResponse) {
//             println!("success: {:?}", response);
//         }

//         async fn on_fail(&self, error: MaeraError) {
//             println!("fail: {:?}", error);
//         }

//         fn schedule<'a>(&self, scheduler: &'a mut AsyncScheduler) -> &'a mut AsyncJob {
//             scheduler.every(clokwerk::Interval::Seconds(2)).at("00:00")
//         }
//     }

//     #[tokio::test]
//     async fn test_maera() {
//         let jobs = vec![Job {
//             name: "test".to_string(),
//             handler: TestHandler {
//                 name: "test".to_string(),
//             },
//         }];
//         let maera = Maera::new(jobs);
//         maera.start().await.unwrap();
//     }
// }
