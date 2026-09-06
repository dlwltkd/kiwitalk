pub mod chat_on;
pub mod info;
pub mod leave;
pub mod normal;
pub mod open;
pub mod write;

use async_stream::try_stream;
use futures_lite::Stream;
use futures_loco_protocol::session::LocoSession;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    request,
    talk::{channel::ChannelMeta, chat::Chatlog},
    RequestResult,
};

use self::{chat_on::ChatOnChannel, info::ChannelInfo};

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn sync_response_shape(document: &bson::Document) -> String {
    document
        .iter()
        .take(24)
        .map(|(name, value)| {
            let name = if name.len() <= 32
                && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            {
                name.as_str()
            } else {
                "[field]"
            };
            let shape = match value {
                bson::Bson::Array(items) => format!("array({})", items.len()),
                bson::Bson::Document(_) => "object".to_owned(),
                bson::Bson::Null => "null".to_owned(),
                bson::Bson::Boolean(_) => "bool".to_owned(),
                bson::Bson::Int32(_) => "i32".to_owned(),
                bson::Bson::Int64(_) => "i64".to_owned(),
                _ => "value".to_owned(),
            };
            format!("{name}:{shape}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SyncChatResponse {
    #[serde(rename = "isOK")]
    pub is_ok: bool,

    #[serde(
        default,
        rename = "chatLogs",
        deserialize_with = "deserialize_null_default"
    )]
    pub chatlogs: Vec<Chatlog>,

    #[serde(default, rename = "jsi")]
    pub jsi_log_id: Option<i64>,

    #[serde(default, rename = "lastTokenId")]
    pub last_token_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
struct SyncChatRequest {
    #[serde(rename = "chatId")]
    chat_id: i64,

    #[serde(rename = "cur")]
    current_log_id: i64,

    #[serde(rename = "max")]
    max_log_id: i64,

    #[serde(rename = "cnt")]
    count: i32,
}

#[derive(Debug, Serialize)]
struct GetChatLogsRequest<'a> {
    #[serde(rename = "chatIds")]
    chat_ids: Vec<i64>,
    #[serde(rename = "logIds")]
    log_ids: &'a [i64],
}

#[derive(Debug, Deserialize)]
struct GetChatLogsResponse {
    #[serde(
        default,
        rename = "chatLogs",
        deserialize_with = "deserialize_null_default"
    )]
    chatlogs: Vec<Chatlog>,
}

#[derive(Debug, Serialize)]
struct ChatLogsSinceRequest {
    #[serde(rename = "chatIds")]
    chat_ids: [i64; 1],
    sinces: [i64; 1],
}

#[derive(Debug, Deserialize)]
pub struct ChatLogsResponse {
    #[serde(
        default,
        rename = "chatLogs",
        deserialize_with = "deserialize_null_default"
    )]
    pub chatlogs: Vec<Chatlog>,
    pub eof: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpdateChatRequest {
    pub push_alert: Option<bool>,
    pub msg_ttl: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct UpdateChatResponse {
    #[serde(default, rename = "chatLog")]
    pub chatlog: Option<Chatlog>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
struct UpdateChatWireRequest {
    #[serde(rename = "chatId")]
    chat_id: i64,

    #[serde(rename = "pushAlert", skip_serializing_if = "Option::is_none")]
    push_alert: Option<bool>,

    #[serde(rename = "msgTtl", skip_serializing_if = "Option::is_none")]
    msg_ttl: Option<i32>,
}

impl UpdateChatWireRequest {
    fn new(chat_id: i64, request: &UpdateChatRequest) -> Self {
        Self {
            chat_id,
            push_alert: request.push_alert,
            msg_ttl: request.msg_ttl.map(|ttl| if ttl == 0 { -1 } else { ttl }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TalkChannel<'a> {
    pub session: &'a LocoSession,
    pub id: i64,
}

impl<'a> TalkChannel<'a> {
    pub const fn new(session: &'a LocoSession, id: i64) -> Self {
        Self { session, id }
    }

    pub async fn info(self) -> RequestResult<ChannelInfo> {
        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "chatInfo")]
            pub chat_info: ChannelInfo,
        }

        Ok(request!(self.session, "CHATINFO", bson {
            "chatId": self.id,
        }, Response)
        .await?
        .chat_info)
    }

    pub async fn chat_on(self, token: Option<i64>) -> RequestResult<ChatOnChannel> {
        self.chat_on_with(&chat_on::Request::new(token.unwrap_or(0)))
            .await
    }

    pub async fn chat_on_with(self, req: &chat_on::Request) -> RequestResult<ChatOnChannel> {
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(rename = "chatId")]
            chat_id: i64,

            #[serde(flatten)]
            inner: &'a chat_on::Request,
        }

        request!(
            self.session,
            "CHATONROOM",
            &Request {
                chat_id: self.id,
                inner: req,
            },
            _
        )
        .await
    }

    pub async fn write_chat(self, req: &write::Request<'_>) -> RequestResult<write::Response> {
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(rename = "chatId")]
            chat_id: i64,

            #[serde(flatten)]
            inner: &'a write::Request<'a>,
        }

        request!(
            self.session,
            "WRITE",
            &Request {
                chat_id: self.id,
                inner: req,
            },
            _
        )
        .await
    }

    pub async fn forward_chat(self, req: &write::ForwardRequest<'_>) -> RequestResult<Chatlog> {
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(rename = "chatId")]
            chat_id: i64,

            #[serde(flatten)]
            inner: &'a write::ForwardRequest<'a>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "chatLog")]
            chatlog: Chatlog,
        }

        Ok(request!(
            self.session,
            "FORWARD",
            &Request {
                chat_id: self.id,
                inner: req,
            },
            Response
        )
        .await?
        .chatlog)
    }

    pub async fn delete_chat(self, log_id: i64) -> RequestResult<()> {
        request!(self.session, "DELETEMSG", bson {
            "chatId": self.id,
            "logId": log_id,
        })
        .await
    }

    pub async fn leave(self, req: &leave::Request<'_>) -> RequestResult<leave::Response> {
        request!(
            self.session,
            "LEAVE",
            &leave::WireRequest::new(self.id, req),
            leave::Response
        )
        .await
    }

    pub async fn sync_chat_page(
        self,
        current_log_id: i64,
        max_log_id: i64,
        count: i32,
    ) -> RequestResult<SyncChatResponse> {
        let document = request!(
            self.session,
            "SYNCMSG",
            &SyncChatRequest {
                chat_id: self.id,
                current_log_id,
                max_log_id,
                count,
            },
            bson::Document
        )
        .await?;
        log::info!(
            "history response received; fields={}",
            sync_response_shape(&document)
        );
        let response: SyncChatResponse = bson::from_document(document)?;
        log::info!(
            "history response decoded; records={}; complete={}; cursor_zero={}; join_start_after_cursor={}; join_start_within_target={}; token_present={}",
            response.chatlogs.len(), response.is_ok, current_log_id == 0,
            response.jsi_log_id.is_some_and(|id| id > current_log_id),
            response.jsi_log_id.is_some_and(|id| id <= max_log_id),
            response.last_token_id.is_some(),
        );
        Ok(response)
    }

    pub fn sync_chat_stream(
        self,
        current_log_id: i64,
        max_log_id: i64,
        count: i32,
    ) -> impl Stream<Item = RequestResult<Vec<Chatlog>>> + 'a {
        try_stream!({
            let mut current = current_log_id;

            loop {
                let res = self.sync_chat_page(current, max_log_id, count).await?;

                let chatlogs = res.chatlogs;

                if chatlogs.is_empty() {
                    return;
                }

                let next = chatlogs.iter().map(|chatlog| chatlog.log_id).max().unwrap();

                if next <= current {
                    return;
                }

                current = next;
                yield chatlogs;

                if res.is_ok || current >= max_log_id {
                    return;
                }
            }
        })
    }

    pub async fn get_chat_logs(self, log_ids: &[i64]) -> RequestResult<Vec<Chatlog>> {
        if log_ids.is_empty() {
            return Ok(Vec::new());
        }
        let response = request!(
            self.session,
            "GETMSGS",
            &GetChatLogsRequest {
                chat_ids: vec![self.id; log_ids.len()],
                log_ids,
            },
            GetChatLogsResponse
        )
        .await?;
        log::info!(
            "missing chat lookup finished; requested={}; returned={}",
            log_ids.len(),
            response.chatlogs.len()
        );
        Ok(response.chatlogs)
    }

    pub async fn chat_logs_since(self, since: i64) -> RequestResult<ChatLogsResponse> {
        let document = request!(
            self.session,
            "MCHATLOGS",
            &ChatLogsSinceRequest {
                chat_ids: [self.id],
                sinces: [since],
            },
            bson::Document
        )
        .await?;
        log::info!(
            "chat history page received; fields={}",
            sync_response_shape(&document)
        );
        let response: ChatLogsResponse = bson::from_document(document)?;
        log::info!(
            "chat history page decoded; records={}; complete={}; cursor_zero={}",
            response.chatlogs.len(),
            response.eof,
            since == 0
        );
        Ok(response)
    }

    pub async fn update(self, push_alert: bool) -> RequestResult<()> {
        self.update_with(&UpdateChatRequest {
            push_alert: Some(push_alert),
            msg_ttl: None,
        })
        .await
        .map(|_| ())
    }

    pub async fn update_with(self, req: &UpdateChatRequest) -> RequestResult<UpdateChatResponse> {
        request!(
            self.session,
            "UPDATECHAT",
            &UpdateChatWireRequest::new(self.id, req),
            UpdateChatResponse
        )
        .await
    }

    pub async fn set_meta(self, ty: i32, content: &str) -> RequestResult<ChannelMeta> {
        #[derive(Deserialize)]
        struct Response {
            pub meta: ChannelMeta,
        }

        Ok(request!(self.session, "SETMETA", bson {
            "chatId": self.id,
            "type": ty,
            "content": content,
        }, Response)
        .await?
        .meta)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        sync_response_shape, ChatLogsResponse, ChatLogsSinceRequest, GetChatLogsRequest,
        GetChatLogsResponse, SyncChatRequest, SyncChatResponse, UpdateChatRequest,
        UpdateChatWireRequest,
    };

    #[test]
    fn history_since_request_preserves_room_cursor_pair() {
        let document = bson::to_document(&ChatLogsSinceRequest {
            chat_ids: [7],
            sinces: [9_007_199_254_740_993],
        })
        .unwrap();
        assert_eq!(
            document,
            bson::doc! {
                "chatIds": [7_i64],
                "sinces": [9_007_199_254_740_993_i64],
            }
        );
        assert!(bson::from_document::<ChatLogsResponse>(bson::doc! {"chatLogs": []}).is_err());
        let response: ChatLogsResponse = bson::from_document(bson::doc! {
            "chatLogs": bson::Bson::Null, "eof": true,
        })
        .unwrap();
        assert!(response.chatlogs.is_empty());
        assert!(response.eof);
    }

    #[test]
    fn missing_message_request_preserves_long_ids_and_room_alignment() {
        let ids = [9_007_199_254_740_993_i64, 9_007_199_254_740_995];
        let document = bson::to_document(&GetChatLogsRequest {
            chat_ids: vec![7; ids.len()],
            log_ids: &ids,
        })
        .unwrap();
        assert_eq!(
            document,
            bson::doc! {
                "chatIds": [7_i64, 7_i64],
                "logIds": [ids[0], ids[1]],
            }
        );
    }

    #[test]
    fn missing_message_response_accepts_unavailable_records() {
        for document in [bson::doc! {}, bson::doc! { "chatLogs": bson::Bson::Null }] {
            let response: GetChatLogsResponse = bson::from_document(document).unwrap();
            assert!(response.chatlogs.is_empty());
        }
    }

    #[test]
    fn history_diagnostics_report_structure_without_message_or_token_values() {
        let shape = sync_response_shape(&bson::doc! {
            "isOK": false,
            "chatLogs": [{ "message": "private message", "authorId": 123_i64 }],
            "accessToken": "private token",
            "lastTokenId": 987654321_i64,
            "not a schema key": "private field",
        });
        assert_eq!(
            shape,
            "isOK:bool,chatLogs:array(1),accessToken:value,lastTokenId:i64,[field]:value"
        );
    }

    #[test]
    fn android_sync_chat_uses_exact_request_keys() {
        let request = SyncChatRequest {
            chat_id: 1,
            current_log_id: 2,
            max_log_id: 3,
            count: 4,
        };

        let document = bson::to_document(&request).unwrap();
        let keys = document.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(keys, BTreeSet::from(["chatId", "cnt", "cur", "max"]));
        assert_eq!(document.get_i64("chatId").unwrap(), 1);
        assert_eq!(document.get_i64("cur").unwrap(), 2);
        assert_eq!(document.get_i64("max").unwrap(), 3);
        assert_eq!(document.get_i32("cnt").unwrap(), 4);
    }

    #[test]
    fn android_sync_chat_defaults_logs_and_keeps_cursors() {
        let response: SyncChatResponse = bson::from_document(bson::doc! {
            "isOK": true,
            "jsi": 10_i64,
            "lastTokenId": 11_i64,
        })
        .unwrap();

        assert!(response.is_ok);
        assert!(response.chatlogs.is_empty());
        assert_eq!(response.jsi_log_id, Some(10));
        assert_eq!(response.last_token_id, Some(11));
    }

    #[test]
    fn android_sync_chat_accepts_null_logs() {
        let response: SyncChatResponse = bson::from_document(bson::doc! {
            "isOK": false,
            "chatLogs": bson::Bson::Null,
        })
        .unwrap();

        assert!(!response.is_ok);
        assert!(response.chatlogs.is_empty());
    }

    #[test]
    fn android_sync_chat_requires_completion_flag() {
        assert!(bson::from_document::<SyncChatResponse>(bson::doc! {}).is_err());
    }

    #[test]
    fn android_update_chat_omits_absent_fields_and_maps_zero_ttl() {
        let minimal = bson::to_document(&UpdateChatWireRequest::new(
            1,
            &UpdateChatRequest {
                push_alert: None,
                msg_ttl: None,
            },
        ))
        .unwrap();
        assert_eq!(minimal, bson::doc! { "chatId": 1_i64 });

        let with_ttl = bson::to_document(&UpdateChatWireRequest::new(
            1,
            &UpdateChatRequest {
                push_alert: Some(false),
                msg_ttl: Some(0),
            },
        ))
        .unwrap();
        assert_eq!(with_ttl.get_i32("msgTtl").unwrap(), -1);
        assert!(!with_ttl.get_bool("pushAlert").unwrap());
    }
}
