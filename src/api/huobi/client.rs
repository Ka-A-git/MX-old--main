use super::error::*;
use super::models::{APIErrorResponse, APIResult};
use reqwest;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};
use ring::{digest, hmac};
use serde::Serialize;
use std::collections::BTreeMap;
use tracing::{debug, info};

#[derive(Clone)]
pub struct Client {
    api_key: String,
    secret_key: String,
}

#[derive(Clone)]
pub struct APIKey {
    api_key: String,
    secret_key: String,
}

static HUOBI_API_HOST: &'static str = "api.huobi.pro";

impl Client {
    pub fn new(api_key: &str, secret_key: &str) -> Self {
        Client {
            api_key: api_key.into(),
            secret_key: secret_key.into(),
        }
    }

    pub fn build_request(parameters: &BTreeMap<String, String>) -> String {
        let mut request = String::new();
        for (key, value) in parameters {
            let param = format!("{}={}&", key, value);
            request.push_str(param.as_ref());
        }
        request.pop(); // remove last &

        request
    }

    pub fn get(&self, endpoint: &str, parameters: &BTreeMap<String, String>) -> APIResult<String> {
        let mut request_o = String::new();
        for (key, value) in parameters {
            let param = format!("{}={}&", key, value);
            request_o.push_str(param.as_ref());
        }
        request_o.pop(); // remove last &

        let request = format!("https://{}{}?{}", HUOBI_API_HOST, endpoint, request_o,);

        let body = reqwest::blocking::get(request.as_str())?.text()?;

        // check for errors
        let err_response: APIErrorResponse<serde_json::Value> =
            serde_json::from_str(body.as_str())?;

        info!("err_response: {:?}", err_response);

        match &err_response.status {
            Some(status) => {
                if status == "error" {
                    return Err(Box::new(HuobiError::ApiError(format!(
                        "result dump: {:?}",
                        err_response
                    ))));
                }
            }
            None => info!("err_response: {:?}", err_response),
        }

        Ok(body)
    }

    pub fn get_signed(
        &self,
        endpoint: &str,
        mut params: BTreeMap<String, String>,
    ) -> APIResult<String> {
        params.insert("AccessKeyId".to_string(), self.api_key.clone());
        params.insert("SignatureMethod".to_string(), "HmacSHA256".to_string());
        params.insert("SignatureVersion".to_string(), "2".to_string());
        params.insert("Timestamp".to_string(), get_timestamp());

        debug!("[Huobi] Make GET request params: {:?}", params);

        let params = build_query_string(params);
        let signature = sign_hmac_sha256_base64(
            &self.secret_key,
            &format!("{}\n{}\n{}\n{}", "GET", HUOBI_API_HOST, endpoint, params,),
        )
        .to_string();

        let request = format!(
            "https://{}{}?{}&Signature={}",
            HUOBI_API_HOST,
            endpoint,
            params,
            percent_encode(&signature.clone())
        );

        debug!("[Huobi] Make GET signed request: {:?}", request);

        let response = reqwest::blocking::get(request.as_str())?;
        let body = response.text()?;

        debug!("[Huobi] GET responce body: {:?}", body);

        // check for errors
        let err_response: APIErrorResponse<serde_json::Value> =
            serde_json::from_str(body.as_str())?;

        match &err_response.status {
            Some(status) => {
                if status == "error" {
                    return Err(Box::new(HuobiError::ApiError(format!(
                        "result dump: {:?}",
                        err_response
                    ))));
                }
            }
            None => info!("err_response: {:?}", err_response),
        }

        Ok(body)
    }

    pub fn post_signed<T: Serialize + ?Sized>(
        &self,
        endpoint: &str,
        mut params: BTreeMap<String, String>,
        payload: &T,
    ) -> APIResult<String> {
        params.insert("AccessKeyId".to_string(), self.api_key.clone());
        params.insert("SignatureMethod".to_string(), "HmacSHA256".to_string());
        params.insert("SignatureVersion".to_string(), "2".to_string());
        params.insert("Timestamp".to_string(), get_timestamp());

        let params = build_query_string(params);
        let signature = sign_hmac_sha256_base64(
            &self.secret_key,
            &format!("{}\n{}\n{}\n{}", "POST", HUOBI_API_HOST, endpoint, params,),
        )
        .to_string();

        let request = format!(
            "https://{}{}?{}&Signature={}",
            HUOBI_API_HOST,
            endpoint,
            params,
            percent_encode(&signature.clone())
        );

        debug!("[Huobi] Make POST signed request: {:?}", request);

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(request.as_str())
            .headers(build_headers(true)?)
            .json(&payload)
            .send();

        let body = response?.text()?;

        debug!("[Huobi] POST responce body: {:?}", body.clone());

        // check for errors
        let err_response: APIErrorResponse<serde_json::Value> =
            serde_json::from_str(body.as_str())?;

        debug!("err_response: {:?}", err_response);

        match &err_response.status {
            Some(status) => {
                if status == "error" {
                    return Err(Box::new(HuobiError::ApiError(format!(
                        "result dump: {:?}",
                        err_response
                    ))));
                }
            }
            None => info!("err_response: {:?}", err_response),
        }

        Ok(body)
    }
}

pub fn build_query_string(parameters: BTreeMap<String, String>) -> String {
    parameters
        .into_iter()
        .map(|(key, value)| format!("{}={}", key, percent_encode(&value.clone())))
        .collect::<Vec<String>>()
        .join("&")
}

pub fn sign_hmac_sha256_base64(secret: &str, digest: &str) -> String {
    use data_encoding::BASE64;

    let signed_key = hmac::SigningKey::new(&digest::SHA256, secret.as_bytes());
    let signature = hmac::sign(&signed_key, digest.as_bytes());
    let b64_encoded_sig = BASE64.encode(signature.as_ref());

    b64_encoded_sig
}

pub fn percent_encode(source: &str) -> String {
    use percent_encoding::{define_encode_set, utf8_percent_encode, USERINFO_ENCODE_SET};
    define_encode_set! {
        pub CUSTOM_ENCODE_SET = [USERINFO_ENCODE_SET] | { '+', ',' }
    }
    let signature = utf8_percent_encode(&source, CUSTOM_ENCODE_SET).to_string();
    signature
}

pub fn get_timestamp() -> String {
    let utc_time = chrono::Utc::now();
    let formatted_time = utc_time.format("%Y-%m-%dT%H:%M:%S").to_string();

    formatted_time
}

pub fn build_headers(post_method: bool) -> APIResult<HeaderMap> {
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
