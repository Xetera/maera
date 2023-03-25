use async_trait::async_trait;
use maera::*;
use serde_json::{from_str, to_string_pretty, Value};
use std::time::Duration;

struct Peet;

#[async_trait]
impl JobHandler for Peet {
    fn request(&self, builder: ChainableRequestBuilder) -> ChainableRequest {
        builder.url("/api/all").build()
    }
    fn wait(&self) -> Duration {
        Duration::from_secs(60)
    }
    async fn on_success(&self, response: &mut MaeraResponse) -> Decision {
        // get JSON from response text
        let body = response.text().await.unwrap();
        let json = from_str::<Value>(&body).unwrap();
        println!("{}", to_string_pretty(&json).unwrap());
        Decision::Continue
    }
}

#[tokio::main]
async fn main() {
    let job = JobBuilder::new()
        .base_url("https://tls.peet.ws")
        .handler(Peet)
        .build();
    let maera = Maera::new(vec![job]);

    maera.start().await.unwrap();
}
