pub mod channel;
pub mod create;
pub mod get_trailer;
pub mod load_channel_list;
pub mod login;
pub mod user;

use futures_loco_protocol::{loco_protocol::command::Header, session::LocoSession};
use serde::Serialize;

use crate::{request, RequestResult};
use async_stream::try_stream;
use futures_lite::Stream;

use self::channel::{normal::TalkNormalChannel, open::TalkOpenChannel, TalkChannel};

#[derive(Serialize)]
struct MessageAcknowledgement {
    #[serde(rename = "notiRead")]
    noti_read: bool,
}

fn message_acknowledgement_body(noti_read: bool) -> Result<Vec<u8>, bson::ser::Error> {
    bson::to_vec(&MessageAcknowledgement { noti_read })
}

#[derive(Debug, Clone, Copy)]
pub struct TalkSession<'a>(pub &'a LocoSession);

impl<'a> TalkSession<'a> {
    pub async fn ping(self) -> RequestResult<()> {
        request!(self.0, "PING", bson {}).await
    }

    pub async fn set_status(self, status: i32) -> RequestResult<()> {
        request!(self.0, "SETST", bson { "st": status }).await
    }

    /// Acknowledge a server `MSG` push with the same packet header.
    pub async fn acknowledge_message(self, header: Header, noti_read: bool) -> RequestResult<()> {
        self.0
            .respond(header, message_acknowledgement_body(noti_read)?)
            .await
            .map_err(|_| crate::RequestError::Write(std::io::ErrorKind::UnexpectedEof.into()))
    }

    pub async fn create_normal_channel(
        self,
        req: create::Request<'a>,
    ) -> RequestResult<create::ResponseVariant> {
        request!(self.0, "CREATE", &req, {
            0 => create::ResponseVariant::Done,
            -310 => create::ResponseVariant::Exists,
        })
        .await
    }

    pub async fn create_memo_channel(self) -> RequestResult<create::ResponseVariant> {
        request!(self.0, "CREATE", bson {
            "memberIds": [],
            "memoChat": true,
        }, {
            0 => create::ResponseVariant::Done,
            -310 => create::ResponseVariant::Exists,
        })
        .await
    }

    pub async fn login(
        self,
        req: login::Request<'a>,
    ) -> RequestResult<(
        login::Response,
        Option<impl Stream<Item = RequestResult<load_channel_list::Response>> + 'a>,
    )> {
        self.login_with_response(req, login::ResponseType::Desktop)
            .await
    }

    pub async fn login_with_response(
        self,
        req: login::Request<'a>,
        response_type: login::ResponseType,
    ) -> RequestResult<(
        login::Response,
        Option<impl Stream<Item = RequestResult<load_channel_list::Response>> + 'a>,
    )> {
        let res = match response_type {
            login::ResponseType::Desktop => {
                request!(self.0, "LOGINLIST", &req, login::Response).await?
            }
            login::ResponseType::AndroidSubdevice => {
                request!(self.0, "LOGINLIST", &req, login::AndroidResponse)
                    .await?
                    .into()
            }
        };

        if res.chat_list.eof {
            return Ok((res, None));
        }

        let chat_ids = req.chat_list.chat_ids;
        let max_ids = req.chat_list.max_ids;
        let mut last_token_id = res.chat_list.last_token_id.unwrap_or(0);
        let mut last_chat_id = res.chat_list.last_chat_id;

        let stream = try_stream!({
            loop {
                let res = match response_type {
                    login::ResponseType::Desktop => {
                        let req = load_channel_list::Request {
                            chat_ids,
                            max_ids,
                            last_token_id,
                            last_chat_id,
                        };
                        request!(self.0, "LCHATLIST", &req, load_channel_list::Response).await?
                    }
                    login::ResponseType::AndroidSubdevice => {
                        let req = load_channel_list::PageRequest {
                            chat_ids,
                            max_ids,
                            last_token_id,
                            last_chat_id: last_chat_id.unwrap_or(0),
                        };
                        request!(self.0, "LCHATLIST", &req, {
                            0 | -310 => |response: load_channel_list::AndroidResponse| response.into(),
                        })
                        .await?
                    }
                };

                if let Some(id) = res.last_token_id {
                    last_token_id = id;
                }
                last_chat_id = res.last_chat_id;

                let eof = res.eof;

                yield res;

                if eof {
                    break;
                }
            }
        });

        Ok((res, Some(stream)))
    }

    pub async fn get_trailer(
        self,
        chat_type: i32,
        key: &str,
    ) -> RequestResult<get_trailer::Response> {
        self.get_trailer_with(&get_trailer::Request {
            token: key,
            log_type: chat_type,
            chat_id: None,
            resource_type: None,
        })
        .await
    }

    pub async fn get_trailer_with(
        self,
        req: &get_trailer::Request<'_>,
    ) -> RequestResult<get_trailer::Response> {
        request!(self.0, "GETTRAILER", req, get_trailer::Response).await
    }

    pub const fn channel(self, id: i64) -> TalkChannel<'a> {
        TalkChannel {
            session: self.0,
            id,
        }
    }

    pub const fn normal_channel(self, id: i64) -> TalkNormalChannel<'a> {
        TalkNormalChannel(self.channel(id))
    }

    pub const fn open_channel(self, id: i64, link_id: i64) -> TalkOpenChannel<'a> {
        TalkOpenChannel::new(self.channel(id), link_id)
    }
}

#[cfg(test)]
mod tests {
    use bson::doc;

    use super::*;

    #[test]
    fn message_acknowledgement_contains_only_current_noti_read_field() {
        let body = message_acknowledgement_body(true).unwrap();

        assert_eq!(
            bson::from_slice::<bson::Document>(&body).unwrap(),
            doc! {
                "notiRead": true,
            }
        );
    }
}
