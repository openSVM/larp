use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

pub fn new_client() -> ClientWithMiddleware {
    #[cfg(feature = "tee_requests")]
    {
        ClientBuilder::new(reqwest::Client::new())
            .with(crate::tee_middleware::TeeMiddleware::new())
            .build()
    }

    #[cfg(not(feature = "tee_requests"))]
    {
        ClientBuilder::new(reqwest::Client::new()).build()
    }
}
