use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};
use serde::Serialize;
use std::collections::BTreeMap;
use tracing::debug;

static STUB_EXCHANGE_API_HOST: &'static str = "localhost:8080";

#[derive(Clone)]
pub struct Client {}

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    pub fn get(
        &self,
        endpoint: &str,
        params: BTreeMap<String, String>,
    ) -> Result<String, &'static str> {
        debug!("[Stub Exchange] Make GET request params: {:?}", params);

        let params = build_query_string(params);

        let request = format!("https://{}{}?{}", STUB_EXCHANGE_API_HOST, endpoint, params,);

        debug!("[Stub Exchange] Make GET request: {:?}", request);

        let response = reqwest::blocking::get(request.as_str()).unwrap();
        let body = response.text().unwrap();

        debug!("[Stub Exchange] GET responce body: {:?}", body);

        Ok(body)
    }

    pub fn post<T: Serialize + ?Sized>(
        &self,
        endpoint: &str,
        params: BTreeMap<String, String>,
        payload: &T,
    ) -> Result<String, &'static str> {
        let params = build_query_string(params);

        let request = format!("https://{}{}?{}", STUB_EXCHANGE_API_HOST, endpoint, params,);

        debug!("[Stub Exchange] Make POST request: {:?}", request);

        let client = reqwest::blocking::Client::new();

        let response = client
            .post(request.as_str())
            .headers(build_headers(true)?)
            .json(&payload)
            .send();

        let body = response.unwrap().text().unwrap();

        debug!("[Stub Exchange] POST responce body: {:?}", body.clone());

        Ok(body)
    }
}

pub fn percent_encode(source: &str) -> String {
    use percent_encoding::{define_encode_set, utf8_percent_encode, USERINFO_ENCODE_SET};
    define_encode_set! {
        pub CUSTOM_ENCODE_SET = [USERINFO_ENCODE_SET] | { '+', ',' }
    }
    let signature = utf8_percent_encode(&source, CUSTOM_ENCODE_SET).to_string();
    signature
}

pub fn build_query_string(parameters: BTreeMap<String, String>) -> String {
    parameters
        .into_iter()
        .map(|(key, value)| format!("{}={}", key, percent_encode(&value.clone())))
        .collect::<Vec<String>>()
        .join("&")
}

pub fn build_headers(post_method: bool) -> Result<HeaderMap, &'static str> {
    let mut custom_headers = HeaderMap::new();

    custom_headers.insert(USER_AGENT, HeaderValue::from_static("rs"));
    if post_method {
        custom_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        custom_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    } else {
        custom_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
    }

    Ok(custom_headers)
}
