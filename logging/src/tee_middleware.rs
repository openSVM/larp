use http::Extensions;
use reqwest::{Client, Request, Response};
use reqwest_middleware::{Middleware, Next, Result};
use std::env;
use tokio;

pub fn tee_server_url() -> String {
    env::var("AIDE_TEE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

pub struct TeeMiddleware {
    tee_client: Client,
}

impl TeeMiddleware {
    pub fn new() -> Self {
        Self {
            tee_client: Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for TeeMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let path = req.url().path();
        let full_url = format!("{}/sidecar_request{}", tee_server_url(), path);

        let body = req.try_clone().and_then(|req| {
            req.body().map(|body| {
                body.as_bytes()
                    .map_or_else(|| Vec::new(), |bytes| bytes.to_vec())
            })
        });

        let endpoint = req
            .url()
            .host_str()
            .map(|host| {
                if let Some(port) = req.url().port() {
                    format!("{}:{}", host, port)
                } else {
                    host.to_string()
                }
            })
            .unwrap_or_else(|| "none".to_string());
        let mut headers = req.headers().clone();
        headers.append("endpoint", endpoint.parse().unwrap());

        let tee_request = self
            .tee_client
            .request(req.method().clone(), full_url)
            .headers(headers);

        let tee_request = if let Some(body) = body {
            tee_request.body(body)
        } else {
            tee_request
        };

        // Spawn the tee request into a separate non-blocking task, we don't care about
        // its result
        tokio::spawn(async move {
            let _ = tee_request.send().await;
        });

        next.run(req, extensions).await
    }
}
