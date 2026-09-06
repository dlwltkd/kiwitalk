use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    ops::Bound,
    path::{Path, PathBuf},
    pin::pin,
    process::ExitCode,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::{Future, TryStreamExt};
use headless_talk::{
    init::{
        config::{ClientEnv, NetworkType},
        Credential, TalkInitializer,
    },
    ClientStatus,
};
use kiwi_talk_api::protocol::ACTIVE_PROTOCOL_PROFILE;
use kiwi_talk_client::conn::{checkin, create_secure_stream};
use kiwi_talk_system::derive_android_subdevice_uuid;
use reqwest::{redirect::Policy, Client, Url};
use talk_api_internal::{
    auth::{android::login as android_login, AccountForm},
    ApiError,
};
use talk_loco_client::{
    futures_loco_protocol::{
        session::{LocoSession, LocoSessionStream},
        LocoClient,
    },
    talk::session::{
        channel::chat_on::ChatOnChannelType,
        load_channel_list::{self, ChannelListType},
        login, TalkSession,
    },
    RequestError,
};
use zeroize::{Zeroize, Zeroizing};

const APP_IDENTIFIER: &str = "org.kiwitalk.kiwitalk";
const SERVICE_URL: &str = "https://katalk.kakao.com";

struct Credentials {
    email: Zeroizing<String>,
    password: Zeroizing<String>,
}

struct ProbeDatabase(PathBuf);

impl ProbeDatabase {
    fn new() -> Result<Self, Box<dyn Error>> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = env::temp_dir().join(format!(
            "kiwitalk-live-probe-{}-{nonce}.sqlite3",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        drop(options.open(&path)?);

        Ok(Self(path))
    }

    fn url(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for ProbeDatabase {
    fn drop(&mut self) {
        let base = self.0.to_string_lossy();
        for path in [
            self.0.clone(),
            PathBuf::from(format!("{base}-shm")),
            PathBuf::from(format!("{base}-wal")),
        ] {
            let _ = fs::remove_file(path);
        }
    }
}

fn sanitized_request_error(context: &str, error: RequestError) -> Box<dyn Error> {
    let detail = match error {
        RequestError::Status(code) => format!("server status {code}"),
        RequestError::Serialize(_) => "request serialization failed".to_owned(),
        RequestError::Read(error) | RequestError::Write(error) => {
            format!("transport error ({:?})", error.kind())
        }
        RequestError::Deserialize(error) => {
            let detail = error
                .to_string()
                .chars()
                .filter(|character| !character.is_control())
                .take(160)
                .collect::<String>();
            format!("response schema mismatch ({detail})")
        }
    };

    format!("{context}: {detail}").into()
}

fn repository_root() -> Result<&'static Path, Box<dyn Error>> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .ok_or_else(|| "could not locate the repository root".into())
}

#[cfg(unix)]
fn ensure_private_regular_file(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| "could not inspect a required private file")?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("a required private file is missing or has unsafe permissions".into());
    }

    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_regular_file(path: &Path) -> Result<(), Box<dyn Error>> {
    if !path.is_file() {
        return Err("a required private file is missing".into());
    }
    Ok(())
}

fn load_credentials() -> Result<Credentials, Box<dyn Error>> {
    let path = repository_root()?.join(".env");
    ensure_private_regular_file(&path)?;
    let entries =
        dotenvy::from_path_iter(path).map_err(|_| "could not load the repository .env")?;
    let mut email = None;
    let mut password = None;

    for entry in entries {
        let (key, value) = entry.map_err(|_| "could not parse the repository .env")?;
        match key.as_str() {
            "kakaoemail" => email = Some(value),
            "kakaopw" => password = Some(Zeroizing::new(value)),
            _ => {}
        }
    }

    Ok(Credentials {
        email: email
            .filter(|value| !value.is_empty())
            .map(Zeroizing::new)
            .ok_or("kakaoemail is missing or empty in .env")?,
        password: password
            .filter(|value| !value.is_empty())
            .ok_or("kakaopw is missing or empty in .env")?,
    })
}

fn device_uuid_path() -> Result<PathBuf, Box<dyn Error>> {
    let data_dir = if let Some(path) = env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path)
    } else {
        PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?).join(".local/share")
    };
    Ok(data_dir.join(APP_IDENTIFIER).join("device_uuid"))
}

fn android_device_uuid() -> Result<String, Box<dyn Error>> {
    let path = device_uuid_path()?;
    ensure_private_regular_file(&path)?;
    let bytes = Zeroizing::new(
        std::fs::read(path).map_err(|_| "could not read the private KiwiTalk device UUID")?,
    );
    if bytes.len() != 64 {
        return Err("the KiwiTalk device UUID does not contain exactly 64 bytes".into());
    }

    let mut seed = Zeroizing::new([0_u8; 64]);
    seed.copy_from_slice(bytes.as_slice());
    Ok(derive_android_subdevice_uuid(&seed))
}

async fn drive_session<S, F>(
    stream: &mut LocoSessionStream<S>,
    request: F,
) -> Result<F::Output, Box<dyn Error>>
where
    S: futures::AsyncRead + futures::AsyncWrite + Unpin,
    F: Future,
{
    let mut request = pin!(request);
    loop {
        tokio::select! {
            response = &mut request => return Ok(response),
            incoming = stream.try_next() => {
                match incoming {
                    Ok(Some(_command)) => {}
                    Ok(None) => return Err("LOCO session ended before LOGINLIST completed".into()),
                    Err(_) => return Err("LOCO session failed before LOGINLIST completed".into()),
                }
            }
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let profile = ACTIVE_PROTOCOL_PROFILE;
    let credentials = load_credentials()?;
    let device_uuid = android_device_uuid()?;
    let http = Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(Policy::none())
        .https_only(true)
        .no_proxy()
        .build()?;
    let auth = profile
        .api_profile()
        .auth_client(&device_uuid, Url::parse(SERVICE_URL)?, http);
    let account = AccountForm {
        email: credentials.email.as_str(),
        password: credentials.password.as_str(),
    };
    let login_result = android_login(auth, account).await;
    drop(credentials);
    let login = match login_result {
        Ok(login) => login,
        Err(ApiError::Status(code)) => {
            return Err(format!("Android authentication was rejected with status {code}").into())
        }
        Err(ApiError::Request(_)) => return Err("Android authentication transport failed".into()),
    };
    let user_id = i64::try_from(login.user_id)?;
    let access_token = Zeroizing::new(login.access_token);
    let mut refresh_token = login.refresh_token;
    refresh_token.zeroize();

    let checkin = tokio::time::timeout(Duration::from_secs(60), checkin(user_id, profile))
        .await
        .map_err(|_| "CHECKIN timed out")?
        .map_err(|_| "CHECKIN failed")?;
    let port = u16::try_from(checkin.port)?;
    println!("CHECKIN accepted");

    let stream = tokio::time::timeout(
        Duration::from_secs(20),
        create_secure_stream((checkin.host.as_str(), port)),
    )
    .await
    .map_err(|_| "LOCO connection timed out")?
    .map_err(|_| "LOCO connection failed")?;
    let client = LocoClient::new(stream);
    let (session, mut stream) = LocoSession::new(client);
    let empty = Vec::<i64>::new();
    let request = login::Request {
        os: profile.os(),
        net_type: NetworkType::Wired as i16,
        app_version: profile.app_version(),
        mccmnc: profile.mccmnc(),
        protocol_version: profile.protocol_version(),
        device_uuid: &device_uuid,
        oauth_token: access_token.as_str(),
        language: profile.language(),
        device_type: profile.device_type(),
        revision: profile.revision(),
        rp: [0x00, 0x00, 0xff, 0xff, 0x00, 0x00],
        pc_status: None,
        chat_list: load_channel_list::Request {
            chat_ids: &empty,
            max_ids: &empty,
            last_token_id: 0,
            last_chat_id: profile.last_chat_id(),
        },
        last_block_token: 0,
        background: profile.background(),
        is_switching: profile.is_switching(),
    };
    let login_result = tokio::time::timeout(
        Duration::from_secs(45),
        drive_session(
            &mut stream,
            TalkSession(&session)
                .login_with_response(request, login::ResponseType::AndroidSubdevice),
        ),
    )
    .await
    .map_err(|_| "LOGINLIST timed out")?
    .map_err(|_| "LOCO session ended before LOGINLIST completed")?;
    let (response, _) = login_result.map_err(|_| "LOGINLIST was rejected or unreadable")?;
    if response.user_id != user_id {
        return Err("LOGINLIST returned a different user identity".into());
    }

    println!(
        "LOGINLIST accepted; {} channel summaries returned",
        response.chat_list.chat_datas.len()
    );

    let named_summaries = response
        .chat_list
        .chat_datas
        .iter()
        .filter(|channel| {
            channel
                .icon_user_nicknames
                .as_ref()
                .is_some_and(|names| names.iter().any(|name| !name.trim().is_empty()))
        })
        .count();
    println!("LOGINLIST included fallback names for {named_summaries} summaries");

    let Some((channel_id, last_seen_log_id)) = response
        .chat_list
        .chat_datas
        .iter()
        .find(|channel| {
            channel.last_log_id > 0
                && matches!(
                    &channel.channel_type,
                    ChannelListType::DirectChat
                        | ChannelListType::MultiChat
                        | ChannelListType::MemoChat
                )
        })
        .map(|channel| (channel.id, channel.last_seen_log_id))
    else {
        println!("CHATONROOM/SYNCMSG probe skipped; no normal room with history was available");
        return Ok(());
    };

    let room = tokio::time::timeout(
        Duration::from_secs(20),
        drive_session(
            &mut stream,
            TalkSession(&session)
                .channel(channel_id)
                .chat_on(last_seen_log_id),
        ),
    )
    .await
    .map_err(|_| "CHATONROOM timed out")?
    .map_err(|_| "LOCO session ended during CHATONROOM")?
    .map_err(|error| sanitized_request_error("CHATONROOM failed", error))?;
    let normal_room = matches!(
        room.channel_type,
        ChatOnChannelType::DirectChat(_)
            | ChatOnChannelType::MultiChat(_)
            | ChatOnChannelType::MemoChat(_)
    );
    println!("CHATONROOM accepted; normal_room={normal_room}");

    if !normal_room || room.last_log_id <= 0 {
        println!("SYNCMSG probe skipped; the selected room had no supported history target");
        return Ok(());
    }

    let mut cursor = 0;
    let mut page_count = 0;
    let mut record_count = 0;
    let mut channel_consistent = true;
    let mut stop_reason = "pageLimit";

    for _ in 0..20 {
        let history = tokio::time::timeout(
            Duration::from_secs(20),
            drive_session(
                &mut stream,
                TalkSession(&session).channel(channel_id).sync_chat_page(
                    cursor,
                    room.last_log_id,
                    50,
                ),
            ),
        )
        .await
        .map_err(|_| "SYNCMSG timed out")?
        .map_err(|_| "LOCO session ended during SYNCMSG")?
        .map_err(|error| sanitized_request_error("SYNCMSG failed", error))?;
        page_count += 1;

        let chatlogs = history.chatlogs;
        channel_consistent &= chatlogs
            .iter()
            .all(|chatlog| chatlog.channel_id == channel_id);
        record_count += chatlogs.len();

        if chatlogs.is_empty() {
            stop_reason = if history.is_ok {
                "serverComplete"
            } else {
                "emptyBatch"
            };
            break;
        }

        let next_cursor = chatlogs.iter().map(|chatlog| chatlog.log_id).max().unwrap();
        if next_cursor <= cursor {
            stop_reason = "noProgress";
            break;
        }
        cursor = next_cursor;

        if history.is_ok {
            stop_reason = "serverComplete";
            break;
        }
        if cursor >= room.last_log_id {
            stop_reason = "reachedTarget";
            break;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!(
        "SYNCMSG accepted; pages={page_count}; records={record_count}; stop={stop_reason}; channel_consistent={}",
        channel_consistent,
    );

    drop(response);
    drop(stream);
    drop(session);

    let database = ProbeDatabase::new()?;
    let core_stream = tokio::time::timeout(
        Duration::from_secs(20),
        create_secure_stream((checkin.host.as_str(), port)),
    )
    .await
    .map_err(|_| "core LOCO connection timed out")?
    .map_err(|_| "core LOCO connection failed")?;
    let initializer = TalkInitializer::new(
        LocoClient::new(core_stream),
        ClientEnv {
            os: profile.os(),
            net_type: NetworkType::Wired,
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
            login_response_type: login::ResponseType::AndroidSubdevice,
        },
        database.url(),
    )
    .await
    .map_err(|_| "core database initialization failed")?;
    let talk = tokio::time::timeout(
        Duration::from_secs(60),
        initializer.login(
            Credential {
                access_token: access_token.as_str(),
                device_uuid: &device_uuid,
            },
            ClientStatus::Unlocked,
            |_| async {},
        ),
    )
    .await
    .map_err(|_| "core LOGINLIST timed out")?
    .map_err(|_| "core LOGINLIST or channel persistence failed")?;

    let visible_channels = talk
        .channel_list()
        .await
        .map_err(|_| "core channel-list database read failed")?;
    println!(
        "CORE initialized; {} supported channel rows are visible",
        visible_channels.len()
    );
    let Some((selected_channel_id, _)) = visible_channels.first() else {
        return Err("core initialized without any supported channel rows".into());
    };
    let selected_channel_id = *selected_channel_id;

    let metadata_loaded = tokio::time::timeout(
        Duration::from_secs(20),
        talk.load_channel(selected_channel_id),
    )
    .await
    .map_err(|_| "core selected-room metadata timed out")?
    .map_err(|_| "core selected-room metadata failed")?
    .is_some();
    let sync = tokio::time::timeout(
        Duration::from_secs(20),
        talk.sync_channel_history(selected_channel_id),
    )
    .await
    .map_err(|_| "core selected-room history timed out")?
    .map_err(|_| "core selected-room history failed")?;
    let cached_messages = talk
        .channel(selected_channel_id)
        .load_chat_from(Bound::Unbounded, 200)
        .await
        .map_err(|_| "core transcript database read failed")?;
    println!(
        "CORE selected room; metadata_loaded={metadata_loaded}; synced_records={}; cached_records={}; complete={}; stop={:?}",
        sync.fetched_count,
        cached_messages.len(),
        sync.complete(),
        sync.stop,
    );

    drop(talk);
    drop(database);
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("read-only LOCO probe failed: {error}");
            ExitCode::FAILURE
        }
    }
}
