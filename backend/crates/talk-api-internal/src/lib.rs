pub mod account;
pub mod agent;
pub mod auth;
pub mod client;
pub mod config;
pub mod credential;
pub mod friend;
pub mod profile;

use std::ops::Deref;

use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RequestError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error("HTTP request failed with status {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("HTTP response exceeded the {limit}-byte limit")]
    ResponseTooLarge { limit: usize },
}

pub type RequestResult<T> = Result<T, RequestError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    Request(RequestError),

    #[error("api responded with error. status: {0}")]
    Status(i32),
}

impl<T: Into<RequestError>> From<T> for ApiError {
    fn from(value: T) -> Self {
        Self::Request(value.into())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub(crate) async fn read_response(request: RequestBuilder) -> ApiResult<impl Deref<Target = [u8]>> {
    #[derive(Debug, Clone, Copy, Deserialize)]
    struct ApiStatus {
        pub status: i32,
    }

    let response = request.send().await?;
    let http_status = response.status();
    if !http_status.is_success() {
        return Err(RequestError::HttpStatus(http_status).into());
    }

    let data = response.bytes().await?;

    match serde_json::from_slice::<ApiStatus>(&data)?.status {
        0 => Ok(data),
        status => Err(ApiError::Status(status)),
    }
}

pub(crate) async fn read_structured_response<T: DeserializeOwned>(
    request: RequestBuilder,
) -> ApiResult<T> {
    Ok(serde_json::from_slice(&read_response(request).await?)?)
}

#[cfg(test)]
mod tests {
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    use super::{read_response, ApiError, RequestError};

    async fn request_with_response(status: &str, body: &str) -> reqwest::RequestBuilder {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        reqwest::Client::new().get(format!("http://{address}"))
    }

    #[tokio::test]
    async fn non_success_http_status_is_returned_without_parsing_the_body() {
        let request = request_with_response("503 Service Unavailable", "not JSON").await;

        let error = match read_response(request).await {
            Ok(_) => panic!("expected an HTTP status error"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            ApiError::Request(RequestError::HttpStatus(
                reqwest::StatusCode::SERVICE_UNAVAILABLE
            ))
        ));
    }

    #[tokio::test]
    async fn successful_http_response_still_returns_api_status_errors() {
        let request = request_with_response("200 OK", r#"{"status":-999}"#).await;

        let error = match read_response(request).await {
            Ok(_) => panic!("expected an API status error"),
            Err(error) => error,
        };

        assert!(matches!(error, ApiError::Status(-999)));
    }
}
