pub mod io;

use futures_lite::{AsyncRead, AsyncWrite};
use futures_loco_protocol::{loco_protocol::command::Method, LocoClient};
use serde::{
    de::Deserializer,
    ser::{SerializeMap, Serializer},
    Deserialize, Serialize,
};

use crate::RequestResult;

use self::io::{MediaSink, MediaStream};

use super::request_simple;

#[derive(Debug)]
pub struct MediaClient<T>(LocoClient<T>);

impl<T> MediaClient<T> {
    pub const fn new(client: LocoClient<T>) -> Self {
        Self(client)
    }

    pub fn into_inner(self) -> LocoClient<T> {
        self.0
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> MediaClient<T> {
    pub async fn post(mut self, req: &PostReq<'_>) -> RequestResult<MediaSink<T>> {
        let PostRes { offset } =
            request_simple::<PostRes>(&mut self.0, Method::new("POST").unwrap(), req).await?;

        Ok(MediaSink {
            offset,
            remaining: req.size - offset,
            inner: self.0,
        })
    }

    pub async fn post_multi(mut self, req: &MPostReq<'_>) -> RequestResult<MediaSink<T>> {
        let PostRes { offset } =
            request_simple::<PostRes>(&mut self.0, Method::new("MPOST").unwrap(), req).await?;

        Ok(MediaSink {
            offset,
            remaining: req.size - offset,
            inner: self.0,
        })
    }

    pub async fn download(mut self, req: &DownReq<'_>) -> RequestResult<MediaStream<T>> {
        let DownRes { size } =
            request_simple::<DownRes>(&mut self.0, Method::new("DOWN").unwrap(), req).await?;

        Ok(MediaStream {
            remaining: size,
            inner: self.0.into_inner(),
        })
    }

    pub async fn download_mini(mut self, req: &MiniReq<'_>) -> RequestResult<MediaStream<T>> {
        let DownRes { size } =
            request_simple::<DownRes>(&mut self.0, Method::new("MINI").unwrap(), req).await?;

        Ok(MediaStream {
            remaining: size,
            inner: self.0.into_inner(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostReq<'a> {
    #[serde(rename = "k")]
    pub key: &'a str,

    #[serde(rename = "s")]
    pub size: i64,

    #[serde(rename = "f")]
    pub name: Option<&'a str>,

    #[serde(rename = "t")]
    pub log_type: i32,

    #[serde(rename = "w")]
    pub width: i32,

    #[serde(rename = "h")]
    pub height: i32,

    #[serde(rename = "c")]
    pub channel_id: i64,

    #[serde(rename = "mid")]
    pub message_id: i64,

    #[serde(rename = "ex")]
    pub extra: &'a str,

    #[serde(rename = "sp")]
    pub supplement: Option<&'a str>,

    #[serde(rename = "ns")]
    pub no_seen: bool,

    #[serde(rename = "dt")]
    pub device_type: i32,

    #[serde(rename = "scp")]
    pub scope: i32,

    #[serde(rename = "tid")]
    pub thread_id: Option<i64>,

    #[serde(rename = "featureStat")]
    pub feature_stat: Option<&'a str>,
    pub silence: bool,

    #[serde(flatten)]
    pub client: MediaClientInfo<'a>,
}

impl Serialize for PostReq<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("u", &self.client.user_id)?;
        map.serialize_entry("k", self.key)?;
        map.serialize_entry("t", &self.log_type)?;
        map.serialize_entry("s", &self.size)?;
        map.serialize_entry("c", &self.channel_id)?;
        map.serialize_entry("mid", &self.message_id)?;
        map.serialize_entry("w", &self.width)?;
        map.serialize_entry("h", &self.height)?;
        serialize_client(&mut map, &self.client)?;
        map.serialize_entry("ex", self.extra)?;
        map.serialize_entry("f", &self.name)?;
        map.serialize_entry("sp", &self.supplement)?;
        map.serialize_entry("ns", &self.no_seen)?;
        map.serialize_entry("dt", &self.device_type)?;
        map.serialize_entry("scp", &self.scope)?;
        if let Some(thread_id) = self.thread_id {
            map.serialize_entry("tid", &thread_id)?;
        }
        if let Some(feature_stat) = self.feature_stat.filter(|value| !value.trim().is_empty()) {
            map.serialize_entry("featureStat", feature_stat)?;
        }
        map.serialize_entry("silence", &self.silence)?;
        map.end()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MPostReq<'a> {
    #[serde(rename = "k")]
    pub key: &'a str,

    #[serde(rename = "s")]
    pub size: i64,

    #[serde(rename = "t")]
    pub log_type: i32,

    #[serde(rename = "dt")]
    pub device_type: i32,

    #[serde(rename = "scp")]
    pub scope: i32,

    #[serde(rename = "tid")]
    pub thread_id: Option<i64>,

    #[serde(flatten)]
    pub client: MediaClientInfo<'a>,
}

impl Serialize for MPostReq<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("u", &self.client.user_id)?;
        map.serialize_entry("k", self.key)?;
        map.serialize_entry("t", &self.log_type)?;
        map.serialize_entry("s", &self.size)?;
        serialize_client(&mut map, &self.client)?;
        map.serialize_entry("dt", &self.device_type)?;
        map.serialize_entry("scp", &self.scope)?;
        if let Some(thread_id) = self.thread_id {
            map.serialize_entry("tid", &thread_id)?;
        }
        map.end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostRes {
    pub offset: i64,
}

impl<'de> Deserialize<'de> for PostRes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(default, rename = "o")]
            offset: i32,
        }

        Ok(Self {
            offset: i64::from(Wire::deserialize(deserializer)?.offset),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownReq<'a> {
    #[serde(rename = "k")]
    pub key: &'a str,

    #[serde(rename = "c")]
    pub channel_id: i64,

    #[serde(rename = "o")]
    pub offset: i64,

    #[serde(rename = "rt")]
    pub real_time: bool,

    #[serde(flatten)]
    pub client: MediaClientInfo<'a>,
}

impl Serialize for DownReq<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("u", &self.client.user_id)?;
        map.serialize_entry("k", self.key)?;
        map.serialize_entry("o", &self.offset)?;
        serialize_client(&mut map, &self.client)?;
        map.serialize_entry("c", &self.channel_id)?;
        map.serialize_entry("rt", &self.real_time)?;
        map.end()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MiniReq<'a> {
    #[serde(rename = "k")]
    pub key: &'a str,

    #[serde(rename = "c")]
    pub channel_id: i64,

    #[serde(rename = "o")]
    pub offset: i64,

    #[serde(flatten)]
    pub client: MediaClientInfo<'a>,
}

impl Serialize for MiniReq<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("u", &self.client.user_id)?;
        map.serialize_entry("k", self.key)?;
        map.serialize_entry("o", &self.offset)?;
        serialize_client(&mut map, &self.client)?;
        map.serialize_entry("c", &self.channel_id)?;
        map.end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DownRes {
    pub size: i64,
}

impl<'de> Deserialize<'de> for DownRes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(default, rename = "s")]
            size: i32,
        }

        Ok(Self {
            size: i64::from(Wire::deserialize(deserializer)?.size),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaClientInfo<'a> {
    #[serde(rename = "u")]
    pub user_id: i64,

    #[serde(rename = "os")]
    pub agent: &'a str,

    #[serde(rename = "av")]
    pub app_version: &'a str,

    #[serde(rename = "nt")]
    pub net_type: i32,

    #[serde(rename = "mm")]
    pub mccmnc: &'a str,
}

fn serialize_client<M>(map: &mut M, client: &MediaClientInfo<'_>) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    map.serialize_entry("mm", client.mccmnc)?;
    map.serialize_entry("nt", &client.net_type)?;
    map.serialize_entry("os", client.agent)?;
    map.serialize_entry("av", client.app_version)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> MediaClientInfo<'static> {
        MediaClientInfo {
            user_id: 1,
            agent: "android",
            app_version: "26.7.2",
            net_type: 0,
            mccmnc: "999",
        }
    }

    #[test]
    fn android_post_serializes_current_fields_in_order() {
        let document = bson::to_document(&PostReq {
            key: "token",
            size: 2,
            name: None,
            log_type: 3,
            width: 4,
            height: 5,
            channel_id: 6,
            message_id: 7,
            extra: "{}",
            supplement: None,
            no_seen: false,
            device_type: 0,
            scope: 0,
            thread_id: None,
            feature_stat: Some(" "),
            silence: false,
            client: client(),
        })
        .unwrap();

        assert_eq!(
            document.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "u", "k", "t", "s", "c", "mid", "w", "h", "mm", "nt", "os", "av", "ex", "f", "sp",
                "ns", "dt", "scp", "silence",
            ]
        );
        assert_eq!(document.get_i32("t"), Ok(3));
        assert_eq!(document.get("f"), Some(&bson::Bson::Null));
        assert_eq!(document.get("sp"), Some(&bson::Bson::Null));
        assert!(!document.contains_key("featureStat"));
        assert!(!document.contains_key("rt"));
    }

    #[test]
    fn android_mpost_serializes_context_and_optional_thread() {
        let document = bson::to_document(&MPostReq {
            key: "token",
            size: 2,
            log_type: 3,
            device_type: 0,
            scope: 1,
            thread_id: Some(4),
            client: client(),
        })
        .unwrap();

        assert_eq!(
            document.keys().map(String::as_str).collect::<Vec<_>>(),
            ["u", "k", "t", "s", "mm", "nt", "os", "av", "dt", "scp", "tid"]
        );
        assert_eq!(document.get_i64("tid"), Ok(4));
    }

    #[test]
    fn android_down_and_mini_use_distinct_field_sets() {
        let down = bson::to_document(&DownReq {
            key: "token",
            channel_id: 2,
            offset: 3,
            real_time: true,
            client: client(),
        })
        .unwrap();
        assert_eq!(
            down.keys().map(String::as_str).collect::<Vec<_>>(),
            ["u", "k", "o", "mm", "nt", "os", "av", "c", "rt"]
        );

        let mini = bson::to_document(&MiniReq {
            key: "token",
            channel_id: 2,
            offset: 3,
            client: client(),
        })
        .unwrap();
        assert_eq!(
            mini.keys().map(String::as_str).collect::<Vec<_>>(),
            ["u", "k", "o", "mm", "nt", "os", "av", "c"]
        );
        assert!(!mini.contains_key("w"));
        assert!(!mini.contains_key("h"));
        assert!(!mini.contains_key("rt"));
    }

    #[test]
    fn android_media_response_uses_int32_offset_and_size() {
        let post: PostRes = bson::from_document(bson::doc! { "o": 12_i32 }).unwrap();
        let down: DownRes = bson::from_document(bson::doc! { "s": 34_i32 }).unwrap();

        assert_eq!(post.offset, 12);
        assert_eq!(down.size, 34);
    }
}
