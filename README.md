# Maera

A simple interval-based site monitor that bypasses TLS fingerprinting on sites.

```rs
use async_trait::async_trait;
use maera::*;
use serde_json::{from_str, to_string_pretty, Value};
use std::time::Duration;

struct Peet;

#[async_trait]
impl JobHandler for Peet {
    type Response = MaeraResponse;
    fn request(&self, builder: ChainableRequestBuilder) -> Chain<Self::Response> {
        builder
            .url("/api/all")
            .delay(Duration::from_secs(60))
            .into()
    }
    async fn on_success(&self, response: &mut Self::Response) -> Decision {
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
```

## Impersonating Browsers

If you're experienced with systems programming, you might expect that libcurl-impersonate-chrome.so is statically linked and shipped with the library itself to have fine control over the fingerprints. However, I'm not, and I don't know how to do it! So you're expected to preload it with the following command when running your binary.

Assuming your shared library lives inside `/usr/local/lib`, you can set the following env variables to preload [the required libraries](https://github.com/lwthiker/curl-impersonate/releases/latest)

```
LD_PRELOAD="/usr/local/lib/libcurl-impersonate-chrome.so" CURL_IMPERSONATE=chrome111 ./your/binary
```

It's recommended to not change the headers for the request too much as the order and values of headers are used for fingerprinting purposes, so a lot of them like `user-agent` are hardcoded.

> Warning, I have no idea how to build rust libraries (as you can tell from the type signature of traits). If you run into this and want to improve the API, feel free to open a PR.

## Why Maera?

She's one of the main characters in the [Destiny's Crucible series](https://www.goodreads.com/book/show/30985483-cast-under-an-alien-sun). Highly recommended read!

![Book cover](https://images-na.ssl-images-amazon.com/images/S/compressed.photo.goodreads.com/books/1468198764i/30985483.jpg)
