mod channel;
mod channel_list;
#[cfg(not(feature = "diagnostics"))]
mod conn;
#[cfg(feature = "diagnostics")]
#[doc(hidden)]
pub mod conn;
mod constants;
mod event;
mod handler;

use handler::handle_event;
use kiwi_talk_api::auth::{Credential as ApiCredential, CredentialState};
use parking_lot::RwLock;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    task::Poll,
};
use tauri::{
    generate_handler,
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime,
};

use anyhow::{anyhow, Context};
use futures::future::poll_fn;
use headless_talk::{
    init::{config::ClientEnv, Credential, TalkInitializer},
    ClientStatus, HeadlessTalk,
};
use talk_loco_client::futures_loco_protocol::LocoClient;
use tokio::sync::mpsc;

use kiwi_talk_result::TauriResult;
use kiwi_talk_system::get_system_info;

use conn::checkin;
use constants::TALK_NET_TYPE;

use self::{
    conn::create_secure_stream,
    event::{ClientEvent, EventSender},
};

const EVENT_QUEUE_CAPACITY: usize = 100;

pub async fn init<R: Runtime>(name: &'static str) -> anyhow::Result<TauriPlugin<R>> {
    Ok(Builder::new(name)
        .setup(move |handle, _api| {
            handle.manage::<Client>(Client::new());

            Ok(())
        })
        .invoke_handler(generate_handler![
            created,
            create,
            reconnect,
            destroy,
            next_event,
            channel_list::channel_list,
            channel::load_channel,
            channel::channel_set_active,
            channel::channel_sync_history,
            channel::channel_send_text,
            channel::channel_load_chat,
            channel::channel_load_archive,
            channel::channel_import_archive,
            channel::normal::normal_channel_read_chat,
        ])
        .build())
}

type ClientState<'a> = tauri::State<'a, Client>;

#[tauri::command]
fn created(state: ClientState<'_>) -> bool {
    state.created()
}

#[derive(Clone, Deserialize, Copy)]
enum Status {
    Unlocked,
    Locked,
}

impl From<Status> for ClientStatus {
    fn from(val: Status) -> Self {
        match val {
            Status::Unlocked => ClientStatus::Unlocked,
            Status::Locked => ClientStatus::Locked,
        }
    }
}

#[tauri::command(async)]
async fn create<R: Runtime>(
    app: AppHandle<R>,
    status: Status,
    cred: CredentialState<'_>,
    state: ClientState<'_>,
) -> TauriResult<i32> {
    let Some(credential) = cred.read().as_ref().map(ApiCredential::snapshot) else {
        return Err(anyhow!("not logon").into());
    };

    let user_id =
        i64::try_from(credential.user_id).context("user id exceeds the LOCO integer range")?;

    state
        .create(
            app,
            status.into(),
            Credential {
                access_token: credential.access_token.as_str(),
                device_uuid: &credential.device_uuid,
            },
            user_id,
            credential.profile,
        )
        .await
        .context("cannot create client")?;

    Ok(0)
}

#[tauri::command(async)]
async fn reconnect<R: Runtime>(
    app: AppHandle<R>,
    status: Status,
    cred: CredentialState<'_>,
    state: ClientState<'_>,
) -> TauriResult<i32> {
    let Some(credential) = cred.read().as_ref().map(ApiCredential::snapshot) else {
        return Err(anyhow!("not logon").into());
    };

    let user_id =
        i64::try_from(credential.user_id).context("user id exceeds the LOCO integer range")?;

    state
        .reconnect(
            app,
            status.into(),
            Credential {
                access_token: credential.access_token.as_str(),
                device_uuid: &credential.device_uuid,
            },
            user_id,
            credential.profile,
        )
        .await
        .context("cannot reconnect client")?;

    Ok(0)
}

#[tauri::command(async)]
async fn destroy(state: ClientState<'_>) -> TauriResult<()> {
    state.destroy().await?;

    Ok(())
}

#[tauri::command(async)]
async fn next_event(client: ClientState<'_>) -> TauriResult<Option<ClientEvent>> {
    let generation = client.with(|inner| inner.generation)?;

    if client.with(|inner| inner.event_tx.overflowed())? {
        return Err(anyhow!("client event queue overflowed; reconnect required").into());
    }

    let event = poll_fn(|cx| {
        let mut inner = client.inner.write();
        let Some(inner) = inner.as_mut() else {
            return Poll::Ready(Err(anyhow!("client event stream stopped")));
        };

        if inner.generation != generation {
            return Poll::Ready(Err(anyhow!("client event stream was replaced")));
        }

        match inner.event_rx.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(event) => Poll::Ready(Ok(event)),
        }
    })
    .await?;

    Ok(event.transpose()?)
}

#[derive(Debug)]
struct Inner {
    generation: u64,
    talk: Arc<HeadlessTalk>,
    event_tx: EventSender,
    event_rx: mpsc::Receiver<anyhow::Result<ClientEvent>>,
    message_id_device_hash: i64,
}

#[derive(Debug)]
struct Client {
    inner: RwLock<Option<Inner>>,
    lifecycle: tokio::sync::Mutex<()>,
    next_generation: AtomicU64,
}

impl Client {
    const fn new() -> Self {
        Self {
            inner: RwLock::new(None),
            lifecycle: tokio::sync::Mutex::const_new(()),
            next_generation: AtomicU64::new(1),
        }
    }

    async fn create<R: Runtime>(
        &self,
        app: AppHandle<R>,
        status: ClientStatus,
        credential: Credential<'_>,
        user_id: i64,
        profile: kiwi_talk_api::protocol::ProtocolProfile,
    ) -> anyhow::Result<()> {
        let _lifecycle = self.lifecycle.lock().await;
        if self.created() {
            return Err(anyhow!("client is already created"));
        }

        self.create_locked(app, status, credential, user_id, profile)
            .await
    }

    async fn reconnect<R: Runtime>(
        &self,
        app: AppHandle<R>,
        status: ClientStatus,
        credential: Credential<'_>,
        user_id: i64,
        profile: kiwi_talk_api::protocol::ProtocolProfile,
    ) -> anyhow::Result<()> {
        let _lifecycle = self.lifecycle.lock().await;
        self.shutdown_locked(false).await?;
        self.create_locked(app, status, credential, user_id, profile)
            .await
    }

    async fn create_locked<R: Runtime>(
        &self,
        app: AppHandle<R>,
        status: ClientStatus,
        credential: Credential<'_>,
        user_id: i64,
        profile: kiwi_talk_api::protocol::ProtocolProfile,
    ) -> anyhow::Result<()> {
        let info = get_system_info();
        let message_id_device_hash =
            channel::android_message_id_device_hash(credential.device_uuid);

        let user_dir = info.data_dir.join("userdata").join({
            let mut digest = Sha256::new();

            digest.update("user_");
            digest.update(format!("{user_id}"));

            hex::encode(digest.finalize())
        });

        tokio::fs::create_dir_all(&user_dir)
            .await
            .context("cannot create user directory")?;

        let checkin = checkin(user_id, profile).await.inspect_err(|_| {
            log::warn!("native chat startup failed; stage=checkin");
        })?;

        let loco_port = u16::try_from(checkin.port).context("CHECKIN returned an invalid port")?;

        let stream = create_secure_stream((checkin.host.as_str(), loco_port))
            .await
            .inspect_err(|_| {
                log::warn!("native chat startup failed; stage=secure_stream");
            })
            .context("failed to create secure stream")?;
        let client = LocoClient::new(stream);

        let initializer = TalkInitializer::new(
            client,
            ClientEnv {
                os: profile.os(),
                net_type: TALK_NET_TYPE,
                app_version: profile.app_version(),
                mccmnc: profile.mccmnc(),
                language: profile.language(),
                protocol_version: profile.protocol_version(),
                device_type: profile.device_type(),
                revision: profile.revision(),
                include_pc_status: profile.include_pc_status(),
                background: profile.background(),
                last_chat_id: profile.last_chat_id(),
                is_switching: profile.is_switching(),
                login_response_type:
                    talk_loco_client::talk::session::login::ResponseType::AndroidSubdevice,
            },
            user_dir.join("client.db").to_string_lossy(),
        )
        .await
        .inspect_err(|_| {
            log::warn!("native chat startup failed; stage=local_database");
        })
        .context("failed to login")?;

        let (event_tx, event_rx) = mpsc::channel(EVENT_QUEUE_CAPACITY);
        let event_tx = EventSender::new(event_tx, Arc::new(AtomicBool::new(false)));

        let callback_tx = event_tx.clone();
        let talk = initializer
            .login(credential, status, move |res| {
                let event_tx = callback_tx.clone();
                let app = app.clone();

                async move {
                    match res {
                        Ok(event) => {
                            if let Err(err) = handle_event(&app, event, event_tx.clone()).await {
                                event_tx.enqueue(Err(err));
                            }
                        }

                        Err(err) => {
                            event_tx.enqueue(Err(err.into()));
                        }
                    }
                }
            })
            .await
            .inspect_err(|_| {
                log::warn!("native chat startup failed; stage=loginlist");
            })
            .context("failed to initialize client")?;

        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
        *self.inner.write() = Some(Inner {
            generation,
            talk: Arc::new(talk),
            event_tx,
            event_rx,
            message_id_device_hash,
        });

        Ok(())
    }

    fn created(&self) -> bool {
        self.inner.read().is_some()
    }

    fn with<T>(&self, f: impl FnOnce(&Inner) -> T) -> anyhow::Result<T> {
        Ok(f(self
            .inner
            .read()
            .as_ref()
            .ok_or_else(|| anyhow!("client is not created"))?))
    }

    fn talk(&self) -> anyhow::Result<Arc<HeadlessTalk>> {
        self.with(|inner| inner.talk.clone())
    }

    fn event_sender(&self) -> anyhow::Result<EventSender> {
        self.with(|inner| inner.event_tx.clone())
    }

    fn message_id_device_hash(&self) -> anyhow::Result<i64> {
        self.with(|inner| inner.message_id_device_hash)
    }

    async fn destroy(&self) -> anyhow::Result<()> {
        let _lifecycle = self.lifecycle.lock().await;
        self.shutdown_locked(true).await
    }

    async fn shutdown_locked(&self, require_created: bool) -> anyhow::Result<()> {
        let inner = self.inner.write().take();

        let Some(inner) = inner else {
            return if require_created {
                Err(anyhow!("client is not created"))
            } else {
                Ok(())
            };
        };

        inner.talk.shutdown().await;

        Ok(())
    }
}
