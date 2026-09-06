pub mod user;

use serde::{Deserialize, Serialize};

use crate::{request, RequestResult};

use self::user::User;

use super::{leave, TalkChannel};

#[derive(Debug, Clone, Copy)]
pub struct TalkNormalChannel<'a>(pub TalkChannel<'a>);

impl<'a> TalkNormalChannel<'a> {
    pub async fn add_users(self, users: &[i64]) -> RequestResult<()> {
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(rename = "chatId")]
            chat_id: i64,

            #[serde(rename = "memberIds")]
            users: &'a [i64],
        }

        request!(
            self.0.session,
            "ADDMEM",
            &Request {
                chat_id: self.0.id,
                users
            }
        )
        .await
    }

    pub async fn list_users(self) -> RequestResult<Vec<User>> {
        #[derive(Deserialize)]
        struct Response {
            #[serde(default)]
            pub members: Vec<User>,

            #[serde(rename = "token")]
            pub _token: i64,
        }

        let response = request!(self.0.session, "GETMEM", bson {
            "chatId": self.0.id,
        }, Response)
        .await?;
        Ok(response.members)
    }

    pub async fn users(self, user_ids: &[i64]) -> RequestResult<Vec<User>> {
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(rename = "chatId")]
            pub chat_id: i64,

            #[serde(rename = "memberIds")]
            pub user_ids: &'a [i64],
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "chatId")]
            pub _chat_id: i64,

            #[serde(default)]
            pub members: Vec<User>,
        }

        let response = request!(
            self.0.session,
            "MEMBER",
            &Request {
                chat_id: self.0.id,
                user_ids,
            },
            Response
        )
        .await?;
        Ok(response.members)
    }

    pub async fn leave(self, block: bool, from: &str) -> RequestResult<leave::Response> {
        self.0
            .leave(&leave::Request {
                block,
                from,
                report: false,
                link_id: None,
                silence: false,
            })
            .await
    }
}
