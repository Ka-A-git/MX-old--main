use super::models::*;
use hyper::{body::Buf, Body, Client, Method, Request, Response, Uri};
use hyper_tls::HttpsConnector;
use serde_json;
use std::collections::HashMap;
use tracing::info;

pub use super::account::Account;

const HUOBI_BASE_URL: &str = "https://api.huobi.pro";
pub struct HuobiApi;

impl HuobiApi {
    fn get_uri(endpoint: &str) -> Uri {
        let url = format!("{}{}", HUOBI_BASE_URL, endpoint);
        let uri = url.parse::<hyper::Uri>().unwrap();
        uri
    }

    async fn get_request(endpoint: &str) -> hyper::Result<impl Buf> {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        let uri = HuobiApi::get_uri(endpoint);
        let resp = client.get(uri).await?;

        // while let Some(chunk) = resp.body_mut().data().await {
        //     stdout().write_all(&chunk?).await?;
        // }

        let body = hyper::body::aggregate(resp).await;
        body
    }

    async fn post_request(
        endpoint: &str,
        _header: HashMap<String, String>,
        _body: HashMap<String, String>,
    ) -> hyper::Result<Response<Body>> {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        let uri = HuobiApi::get_uri(endpoint);

        let body = Body::from("");

        let req = Request::builder().method(Method::POST).uri(uri).body(body); //.(key, value);
        let responce = client.request(req.unwrap()).await;

        responce
    }

    /// Huobi API "/v1/common/symbols"
    pub async fn symbols() -> hyper::Result<ResultSymbol> {
        info!("[Huobi] get symbols");
        let endpoint = "/v1/common/symbols";
        let body = HuobiApi::get_request(endpoint).await?;
        let symbols = serde_json::from_reader(body.reader());
        Ok(symbols.unwrap())
    }
}

#[cfg(test)]
mod tests {}
