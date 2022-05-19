use reqwest;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};

pub struct Request {
    base_url: String,
}

impl Request {
    pub fn new(base_url: &str) -> Self {
        Request {
            base_url: base_url.to_string(),
        }
    }

    fn make_url(self, endpoint: &str) -> String {
        format!("{}/{}", self.base_url, endpoint)
    }

    /// Make GET request to Trading Platform
    pub async fn get_request(self, endpoint: &str) -> Result<String, reqwest::Error> {
        let url = self.make_url(endpoint);

        // let body = async {
        // match reqwest::get(&url).await {
        //     Ok(responce) => match responce.text().await {
        //         Ok(text) => text,
        //         Err(e) => {
        //             return reqwest::Error(e);
        //         }
        //     },
        //     Err(e) => {
        //         eprintln!("{}", e);
        //         return Err(e);
        //     }
        // }
        // }
        // .await;
        // Ok(body)

        reqwest::get(&url).await?.text().await
    }

    pub async fn post_request(
        self,
        endpoint: &str,
        // form_data: Form,
        params: &[(&str, &str)],
    ) -> Result<String, reqwest::Error> {
        let headers: HeaderMap<HeaderValue> = HeaderMap::default();
        let client = Client::new();

        let url = self.make_url(endpoint);

        let responce = client
            .post(url.as_str())
            .headers(headers)
            .form(params)
            .send()
            .await?;

        responce.text().await
    }
}
