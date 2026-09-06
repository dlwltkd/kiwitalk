pub mod config;

use std::{collections::VecDeque, io, pin::pin};

use diesel::{QueryDsl, RunQueryDsl};
use futures::{AsyncRead, AsyncWrite, Future, TryStream, TryStreamExt};
use futures_loco_protocol::{
    loco_protocol::command::BoxedCommand,
    session::{LocoSession, LocoSessionStream},
    LocoClient,
};
use talk_loco_client::talk::session::{
    load_channel_list::{self},
    login, TalkSession,
};
use talk_loco_client::RequestError;
use thiserror::Error;
use tokio::time;

use crate::{
    conn::Conn,
    constants::PING_INTERVAL,
    database::{schema::channel_list, DatabasePool, MigrationError, PoolTaskError},
    event::ClientEvent,
    handler::{error::HandlerError, SessionHandler},
    task::BackgroundTask,
    updater::list::ChannelListUpdater,
    ClientError, ClientStatus, HeadlessTalk,
};

use self::config::ClientEnv;

const LOGIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);
const STREAM_COMMAND_BUFFER_LIMIT: usize = 256;

pub struct TalkInitializer<'a, S> {
    session: LocoSession,
    stream: LocoSessionStream<S>,

    pool: DatabasePool,

    env: ClientEnv<'a>,
}

impl<'a, S: AsyncRead + AsyncWrite + Unpin> TalkInitializer<'a, S> {
    pub async fn new(
        client: LocoClient<S>,
        env: ClientEnv<'a>,
        database_url: impl Into<String>,
    ) -> Result<TalkInitializer<'a, S>, InitError> {
        let (session, stream) = LocoSession::new(client);

        let pool = DatabasePool::initialize(database_url)
            .await
            .map_err(|error| {
                log::warn!("native chat startup failed; stage=database_pool");
                error
            })?;
        pool.migrate_to_latest().await.map_err(|error| {
            log::warn!("native chat startup failed; stage=database_migration");
            error
        })?;

        Ok(Self {
            session,
            stream,

            pool,

            env,
        })
    }

    pub async fn login<F, Fut>(
        mut self,
        credential: Credential<'_>,
        status: ClientStatus,
        command_handler: F,
    ) -> Result<HeadlessTalk, LoginError>
    where
        S: Send + 'static,
        F: Fn(Result<ClientEvent, HandlerError>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send + Sync + 'static,
    {
        let mut channel_list = Vec::new();

        let (chat_ids, max_ids) = self
            .pool
            .spawn(|conn| {
                let iter = channel_list::table
                    .select((channel_list::id, channel_list::last_seen_log_id))
                    .load_iter::<(i64, Option<i64>), _>(conn)?;

                let mut chat_ids = Vec::with_capacity(iter.size_hint().0);
                let mut max_ids = Vec::with_capacity(iter.size_hint().0);

                for res in iter {
                    let (channel_id, max_id) = res?;

                    chat_ids.push(channel_id);
                    max_ids.push(max_id.unwrap_or(0));
                }

                Ok((chat_ids, max_ids))
            })
            .await
            .map_err(|error| {
                log::warn!("native chat startup failed; stage=channel_state_load");
                ClientError::from(error)
            })?;

        let mut stream_buffer = VecDeque::new();

        let login_result = time::timeout(
            LOGIN_TIMEOUT,
            run_session(&mut self.stream, &mut stream_buffer, async {
                let (res, stream) = TalkSession(&self.session)
                    .login_with_response(
                        login::Request {
                            os: self.env.os,
                            net_type: self.env.net_type as _,
                            app_version: self.env.app_version,
                            mccmnc: self.env.mccmnc,
                            protocol_version: self.env.protocol_version,
                            device_uuid: credential.device_uuid,
                            oauth_token: credential.access_token,
                            language: self.env.language,
                            device_type: self.env.device_type,
                            pc_status: self.env.include_pc_status.then_some(status as _),
                            revision: self.env.revision,
                            rp: [0x00, 0x00, 0xff, 0xff, 0x00, 0x00],
                            chat_list: load_channel_list::Request {
                                chat_ids: &chat_ids,
                                max_ids: &max_ids,
                                last_token_id: 0,
                                last_chat_id: self.env.last_chat_id,
                            },
                            last_block_token: 0,
                            background: self.env.background,
                        },
                        self.env.login_response_type,
                    )
                    .await?;

                channel_list.push(res.chat_list.chat_datas);

                if let Some(stream) = stream {
                    let mut stream = pin!(stream);

                    while let Some(res) = stream.try_next().await? {
                        channel_list.push(res.chat_datas);
                    }
                }

                Ok::<_, ClientError>((res.user_id, res.chat_list.deleted_chat_ids))
            }),
        )
        .await
        .map_err(|_| {
            log::warn!("native chat startup failed; stage=loginlist_timeout");
            io::Error::new(io::ErrorKind::TimedOut, "LOGINLIST request timed out")
        })?;
        let login_result = login_result.map_err(|error| {
            log::warn!(
                "native chat startup failed; stage=loginlist_stream; io_kind={:?}",
                error.kind()
            );
            error
        })?;
        let (user_id, deleted_channels) = login_result.map_err(|error| {
            log_loginlist_request_error(&error);
            error
        })?;

        let conn = Conn {
            user_id,
            session: self.session.clone(),
            pool: self.pool.clone(),
        };

        let stream_task = BackgroundTask::new(tokio::spawn({
            let handler = SessionHandler::new(conn.clone());

            async move {
                let mut stream = pin!(self.stream);

                loop {
                    let read = if let Some(read) = stream_buffer.pop_front() {
                        read
                    } else {
                        match stream.try_next().await {
                            Ok(Some(read)) => read,
                            Ok(None) => {
                                report_stream_failure(
                                    &command_handler,
                                    io::Error::new(
                                        io::ErrorKind::UnexpectedEof,
                                        "LOCO stream closed",
                                    ),
                                )
                                .await;
                                break;
                            }
                            Err(err) => {
                                report_stream_failure(&command_handler, err).await;
                                break;
                            }
                        }
                    };

                    let result = pump_stream_while(
                        stream.as_mut(),
                        &mut stream_buffer,
                        handler.handle(read),
                    )
                    .await;

                    match result {
                        Ok(result) => {
                            dispatch_handler_result(&command_handler, result).await;
                        }
                        Err(err) => {
                            report_stream_failure(&command_handler, err).await;
                            break;
                        }
                    }
                }
            }
        }));

        let ping_task = BackgroundTask::new(tokio::spawn({
            let session = self.session.clone();

            async move {
                let mut interval = time::interval(PING_INTERVAL);

                while TalkSession(&session).ping().await.is_ok() {
                    interval.tick().await;
                }
            }
        }));

        ChannelListUpdater::new(&self.session, &self.pool)
            .update(
                channel_list.into_iter().flatten(),
                deleted_channels,
                self.env.login_response_type == login::ResponseType::Desktop,
            )
            .await
            .map_err(|error| {
                log::warn!("native chat startup failed; stage=channel_list_update");
                error
            })?;

        Ok(HeadlessTalk {
            conn,
            _ping_task: ping_task,
            _stream_task: stream_task,
        })
    }
}

fn log_loginlist_request_error(error: &ClientError) {
    match error {
        ClientError::Request(RequestError::Status(code)) => log::warn!(
            "native chat startup failed; stage=loginlist_request; kind=status; code={code}"
        ),
        ClientError::Request(RequestError::Serialize(_)) => {
            log::warn!("native chat startup failed; stage=loginlist_request; kind=serialize")
        }
        ClientError::Request(RequestError::Read(error)) => log::warn!(
            "native chat startup failed; stage=loginlist_request; kind=read; io_kind={:?}",
            error.kind()
        ),
        ClientError::Request(RequestError::Write(error)) => log::warn!(
            "native chat startup failed; stage=loginlist_request; kind=write; io_kind={:?}",
            error.kind()
        ),
        ClientError::Request(RequestError::Deserialize(error)) => log::warn!(
            "native chat startup failed; stage=loginlist_request; kind=deserialize; detail={error}"
        ),
        ClientError::Database(_) => {
            log::warn!("native chat startup failed; stage=loginlist_request; kind=database")
        }
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum LoginError {
    Client(#[from] ClientError),
    Io(#[from] io::Error),
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum InitError {
    DatabaseInit(#[from] PoolTaskError),
    Migration(#[from] MigrationError),
}

#[derive(Clone, Copy)]
pub struct Credential<'a> {
    pub access_token: &'a str,
    pub device_uuid: &'a str,
}

async fn run_session<F: Future>(
    stream: &mut LocoSessionStream<impl AsyncRead + AsyncWrite + Unpin>,
    buffer: &mut VecDeque<BoxedCommand>,
    task: F,
) -> Result<F::Output, io::Error> {
    let stream_task = async {
        while let Some(read) = stream.try_next().await? {
            buffer_stream_command(buffer, read)?;
        }

        Ok::<_, io::Error>(())
    };

    tokio::select! {
        res = task => Ok(res),
        res = stream_task => {
            res?;
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "LOCO stream ended during login",
            ))
        },
    }
}

async fn pump_stream_while<S, F>(
    mut stream: std::pin::Pin<&mut S>,
    buffer: &mut VecDeque<BoxedCommand>,
    task: F,
) -> Result<F::Output, io::Error>
where
    S: TryStream<Ok = BoxedCommand, Error = io::Error> + Unpin + ?Sized,
    F: Future,
{
    let mut task = pin!(task);

    loop {
        let mut stream_ref = stream.as_mut();
        let stream_read = stream_ref.try_next();

        tokio::select! {
            result = &mut task => return Ok(result),
            read = stream_read => match read? {
                Some(read) => buffer_stream_command(buffer, read)?,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "LOCO stream closed",
                    ));
                }
            },
        }
    }
}

fn buffer_stream_command(
    buffer: &mut VecDeque<BoxedCommand>,
    command: BoxedCommand,
) -> Result<(), io::Error> {
    if buffer.len() >= STREAM_COMMAND_BUFFER_LIMIT {
        return Err(io::Error::other("LOCO push buffer capacity exceeded"));
    }

    buffer.push_back(command);
    Ok(())
}

async fn report_stream_failure<F, Fut>(command_handler: &F, error: io::Error)
where
    F: Fn(Result<ClientEvent, HandlerError>) -> Fut,
    Fut: Future,
{
    let (level, message) = if is_disconnect(&error) {
        (log::Level::Info, "LOCO stream disconnected")
    } else {
        (log::Level::Warn, "LOCO stream failed")
    };

    log::log!(level, "{message}");
    command_handler(Err(HandlerError::Io(io::Error::new(error.kind(), message)))).await;
}

async fn dispatch_handler_result<F, Fut>(
    command_handler: &F,
    result: Result<Option<ClientEvent>, HandlerError>,
) where
    F: Fn(Result<ClientEvent, HandlerError>) -> Fut,
    Fut: Future,
{
    match result {
        Ok(Some(event)) => {
            command_handler(Ok(event)).await;
        }
        Ok(None) => {}
        Err(err) => {
            log::warn!(
                "ignored LOCO push after a {} handler failure",
                handler_error_category(&err)
            );
        }
    }
}

const fn handler_error_category(error: &HandlerError) -> &'static str {
    match error {
        HandlerError::Client(ClientError::Request(_)) => "request",
        HandlerError::Client(ClientError::Database(_)) => "database",
        HandlerError::Deserialize(_) => "decode",
        HandlerError::Io(_) => "I/O",
    }
}

fn is_disconnect(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::UnexpectedEof
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::NotConnected
    )
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use super::*;
    use futures::{stream, StreamExt};
    use futures_loco_protocol::loco_protocol::command::{Command, Header, Method};

    fn command(id: u32) -> BoxedCommand {
        Command {
            header: Header {
                id,
                status: 0,
                method: Method::new("MSG").unwrap(),
                data_type: 0,
            },
            data: Box::default(),
        }
    }

    #[tokio::test]
    async fn handler_wait_can_progress_while_pushes_are_buffered_in_order() {
        let (stream_polled_tx, stream_polled_rx) = futures::channel::oneshot::channel();
        let mut input = Box::pin(
            stream::once(async move {
                let _ = stream_polled_tx.send(());
                Ok::<_, io::Error>(command(1))
            })
            .chain(stream::pending()),
        );
        let mut buffer = VecDeque::new();

        let output = pump_stream_while(std::pin::Pin::new(&mut input), &mut buffer, async {
            stream_polled_rx.await.unwrap();
            7
        })
        .await
        .unwrap();

        assert_eq!(output, 7);
        assert_eq!(buffer.pop_front().unwrap().header.id, 1);
    }

    #[test]
    fn stream_command_buffer_has_a_hard_limit() {
        let mut buffer = VecDeque::from(vec![command(1); STREAM_COMMAND_BUFFER_LIMIT]);

        assert!(buffer_stream_command(&mut buffer, command(2)).is_err());
        assert_eq!(buffer.len(), STREAM_COMMAND_BUFFER_LIMIT);
    }

    #[tokio::test]
    async fn handler_failure_is_nonfatal_and_event_callback_order_is_preserved() {
        let observed = Arc::new(Mutex::new(Vec::new()));
        let callback = {
            let observed = observed.clone();

            move |result| {
                let observed = observed.clone();

                async move {
                    let label = match result {
                        Ok(ClientEvent::SwitchServer) => {
                            time::sleep(Duration::from_millis(5)).await;
                            "switch"
                        }
                        Ok(ClientEvent::Kickout(_)) => "kickout",
                        Ok(_) => "event",
                        Err(_) => "fatal",
                    };

                    observed.lock().unwrap().push(label);
                }
            }
        };

        let handler_error = HandlerError::Io(io::Error::other("private transport detail"));
        assert_eq!(handler_error_category(&handler_error), "I/O");

        dispatch_handler_result(&callback, Err(handler_error)).await;
        dispatch_handler_result(&callback, Ok(Some(ClientEvent::SwitchServer))).await;
        dispatch_handler_result(&callback, Ok(Some(ClientEvent::Kickout(1)))).await;
        dispatch_handler_result(&callback, Ok(None)).await;

        assert_eq!(*observed.lock().unwrap(), ["switch", "kickout"]);
    }

    #[test]
    fn expected_transport_closures_are_disconnects() {
        for kind in [
            io::ErrorKind::UnexpectedEof,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::ConnectionAborted,
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::NotConnected,
        ] {
            assert!(is_disconnect(&io::Error::from(kind)));
        }

        assert!(!is_disconnect(&io::Error::from(io::ErrorKind::InvalidData)));
    }
}
