use async_trait::async_trait;
use maera::{
    AsyncBody, AsyncJob, AsyncReadResponseExt, AsyncScheduler, Interval, Job, JobHandler, Maera,
    Method,
};
use serde_json::Value;

#[derive(Clone)]
struct Peet;

#[async_trait]
impl JobHandler for Peet {
    fn target(&self) -> maera::MaeraRequest {
        maera::Request::builder()
            .method(Method::GET)
            .uri("https://tls.peet.ws/api/all")
            .body(AsyncBody::empty())
            .unwrap()
    }
    fn schedule<'a>(&self, scheduler: &'a mut AsyncScheduler) -> &'a mut AsyncJob {
        scheduler.every(Interval::Seconds(2))
    }
    async fn on_success(&self, response: &mut maera::MaeraResponse) {
        let text = serde_json::from_str::<Value>(&response.text().await.unwrap()).unwrap();
        println!("{}", serde_json::to_string_pretty(&text).unwrap());
    }
    async fn on_error(&self, error: maera::MaeraError) {
        println!("error: {:?}", error);
    }
}

#[tokio::main]
async fn main() {
    let maera = Maera::new(vec![Job {
        handler: Peet,
        name: "tls.peet.ws".to_owned(),
    }]);

    maera.start().await.unwrap();
}
