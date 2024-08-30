use reqwest::header::{self, HeaderMap};
use thiserror::Error;

// Endpoints list
const WRITE_API: &'static str = "/api/v2/write";

const USER_AGENT: &'static str = "LibRS 0.1.0";

#[derive(Debug, Clone)]
pub enum Precision {
    MS,
    S,
    US,
    NS,
}

impl ToString for Precision {
    fn to_string(&self) -> String {
        return match self {
            Precision::MS => String::from("ms"),
            Precision::S => String::from("s"),
            Precision::US => String::from("us"),
            Precision::NS => String::from("ns"),
        };
    }
}

impl Default for Precision {
    fn default() -> Self {
        return Precision::NS;
    }
}

#[derive(Error, Debug)]
pub enum InfError {
    #[error("http client error: {0}")]
    HttpError(String)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct InfClient {
    server_url: String,

    org: String,
    precision: Precision,

    http_client: HTTPTransport,
}

impl InfClient {
    pub fn new<T>(server_url: T, api_token: T, org: T) -> Self
    where
        T: Into<String>,
    {
        return InfClient {
            server_url: server_url.into(),
            org: org.into(),
            precision: Default::default(),
            http_client: HTTPTransport::new(api_token),
        };
    }

    pub fn precision(mut self, precision: Precision) -> Self {
        self.precision = precision;
        self
    }

    pub fn build(self) -> InfClient {
        InfClient {
            server_url: self.server_url,
            org: self.org,
            precision: self.precision,
            http_client: self.http_client,
        }
    }

    pub async fn write_point<T>(&self, bucket: T, point: T) -> Result<String, InfError>
    where
        T: Into<String>,
    {
        let url = self.get_url(WRITE_API.to_string());

        return self
            .http_client
            .make_post(url, bucket.into(), self.org.clone(), point.into())
            .await;
    }

    fn get_url(&self, endpoint: String) -> String {
        return format!("{}{}", self.server_url, endpoint);
    }
}

#[derive(Debug, Clone)]
struct HTTPTransport {
    http_client: reqwest::Client,
}

impl HTTPTransport {
    pub fn new<T>(auth_token: T) -> Self
    where
        T: Into<String>,
    {
        let mut headers = HeaderMap::new();
        let mut auth =
            header::HeaderValue::from_str(format!("Token {}", auth_token.into()).as_str()).unwrap();
        auth.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth);

        let http_client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()
            .unwrap();

        return HTTPTransport { http_client };
    }

    pub async fn make_post(
        &self,
        url: String,
        bucket: String,
        org: String,
        data: String,
    ) -> Result<String, InfError> {
        let data = self
            .http_client
            .post(url)
            .query(&[("bucket", bucket), ("org", org)])
            .body::<String>(data.into())
            .send()
            .await;

        let resp = match data {
            Ok(resp) => resp, 
            Err(e) => return Err(InfError::HttpError(e.to_string()))
        };

        let txt = match resp.text().await {
            Ok(t) => t,
            Err(e) => return Err(InfError::HttpError(e.to_string()))
        };

        return Ok(txt);
    }
}

#[cfg(test)]
mod tests {
    use crate::InfClient;

    #[tokio::test]
    async fn test_send_data() {
        let token = String::from("");
        let client = InfClient::new(
            String::from("https://eu-central-1-1.aws.cloud2.influxdata.com"),
            token,
            "8abb847537d8fb7c".to_string(),
        );
        let point = "airSensors,sensor_id=TLM0201 temperature=73.97038159354763,humidity=35.23103248356096,co=0.48445310567793615 1724611938000000000";
        let d = client.write_point("test", point).await;

        assert_eq!(String::from(""), d.unwrap());
    }
}
