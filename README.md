# Maera

A simple interval-based site monitor that bypasses TLS fingerprinting on sites.

```rs
use async_trait::async_trait;
use maera::{
    AsyncBody, AsyncJob, AsyncReadResponseExt, AsyncScheduler, Interval, Job, JobHandler, Maera,
    Method,
};
use serde_json::Value;

// Define the struct that represents the site you're monitoring
#[derive(Clone)]
struct Peet;

#[async_trait]
impl JobHandler for Peet {
    // The target website that's going to be requested
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
        // get JSON from response text
        let text = serde_json::from_str::<Value>(&response.text().await.unwrap()).unwrap();
        println!("{}", serde_json::to_string_pretty(&text).unwrap());
    }
    async fn on_error(&self, error: maera::MaeraError) {
        println!("error: {:?}", error);
    }
}

fn main () {
  let maera = Maera::new(vec![Job { handler: Peet, name: "tls.peet.ws" }]);

  maera.start().await.unwrap();
}
```

It's recommended to not change the headers for the request too much as the order and values of headers are used for fingerprinting purposes, so a lot of them like `user-agent` are hardcoded.

> Warning, I have no idea how to build rust libraries (as you can tell from the type signature of traits). If you run into this and want to improve the API, feel free to open a PR.


## What's up with the name maera?

She's one of the main characters in the [Destiny's Crucible series](https://www.goodreads.com/book/show/30985483-cast-under-an-alien-sun). Highly recommended read!

![Book cover](https://images-na.ssl-images-amazon.com/images/S/compressed.photo.goodreads.com/books/1468198764i/30985483.jpg)
