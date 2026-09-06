use crate::{config::Config, credential::Credential, RequestResult};
use reqwest::{header, Client, Method, Request, RequestBuilder};
use url::Url;

#[derive(Clone)]
pub struct ApiClient<'a> {
    credential: Credential<'a>,
    inner: TalkHttpClient<'a>,
}

impl<'a> ApiClient<'a> {
    pub const fn new(credential: Credential<'a>, inner: TalkHttpClient<'a>) -> Self {
        Self { credential, inner }
    }

    pub fn request(self, method: Method, end_point: &str) -> RequestResult<RequestBuilder> {
        Ok(self.inner.request(method, end_point)?.header(
            header::AUTHORIZATION,
            format!(
                "{}-{}",
                self.credential.access_token, self.credential.device_uuid
            ),
        ))
    }

    pub fn request_pilsner(self, method: Method, end_point: &str) -> RequestResult<RequestBuilder> {
        Ok(self.inner.request_pilsner(method, end_point)?.header(
            header::AUTHORIZATION,
            format!(
                "{}-{}",
                self.credential.access_token, self.credential.device_uuid
            ),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct TalkHttpClient<'a> {
    pub config: Config<'a>,
    url: Url,
    pilsner_url: Url,
    client: Client,
}

impl<'a> TalkHttpClient<'a> {
    pub fn new(config: Config<'a>, url: Url, client: Client) -> Self {
        Self::new_with_pilsner_url(
            config,
            url,
            Url::parse("https://talk-pilsner.kakao.com")
                .expect("the built-in Pilsner URL must be valid"),
            client,
        )
    }

    pub fn new_with_pilsner_url(
        config: Config<'a>,
        url: Url,
        pilsner_url: Url,
        client: Client,
    ) -> Self {
        Self {
            config,
            url,
            pilsner_url,
            client,
        }
    }

    pub fn request(self, method: Method, end_point: &str) -> RequestResult<RequestBuilder> {
        let url = self
            .url
            .join(&format!("{}/{}", self.config.agent.agent(), end_point))?;

        Ok(self.request_at(method, url).header(
            "A",
            format!(
                "{}/{}/{}",
                self.config.agent.agent(),
                self.config.version,
                self.config.language
            ),
        ))
    }

    pub fn request_pilsner(self, method: Method, end_point: &str) -> RequestResult<RequestBuilder> {
        let url = self.pilsner_url.join(&format!("talk/{end_point}"))?;

        Ok(self
            .request_at(method, url)
            .header(
                "talk-agent",
                format!("{}/{}", self.config.agent.agent(), self.config.version),
            )
            .header("talk-language", self.config.language))
    }

    fn request_at(&self, method: Method, url: Url) -> RequestBuilder {
        let user_agent = self.config.get_user_agent();

        let host = url.host_str().map(ToString::to_string);

        let mut builder =
            RequestBuilder::from_parts(self.client.clone(), Request::new(method, url))
                .header(header::USER_AGENT, user_agent)
                .header(header::ACCEPT, "*/*")
                .header(header::ACCEPT_LANGUAGE, self.config.language);

        if let Some(host) = host {
            builder = builder.header(header::HOST, host);
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use reqwest::{header, Client, Method};
    use url::Url;

    use crate::{agent::TalkApiAgent, config::Config, credential::Credential};

    use super::{ApiClient, TalkHttpClient};

    #[test]
    fn pilsner_request_uses_its_own_host_and_headers() {
        let http = TalkHttpClient::new_with_pilsner_url(
            Config {
                language: "ko",
                version: "26.7.2",
                agent: TalkApiAgent::Android("16"),
            },
            Url::parse("https://katalk.kakao.com").unwrap(),
            Url::parse("https://talk-pilsner.kakao.com").unwrap(),
            Client::new(),
        );
        let request = ApiClient::new(
            Credential {
                access_token: "access-token",
                device_uuid: "device-uuid",
            },
            http,
        )
        .request_pilsner(Method::GET, "profile25/other")
        .unwrap()
        .query(&[("userId", 123_u64)])
        .build()
        .unwrap();

        assert_eq!(
            request.url().as_str(),
            "https://talk-pilsner.kakao.com/talk/profile25/other?userId=123"
        );
        assert_eq!(request.headers()["talk-agent"], "android/26.7.2");
        assert_eq!(request.headers()["talk-language"], "ko");
        assert_eq!(
            request.headers()[header::AUTHORIZATION],
            "access-token-device-uuid"
        );
        assert_eq!(request.headers()[header::USER_AGENT], "KT/26.7.2 An/16 ko");
        assert!(request.headers().get("A").is_none());
    }
}
