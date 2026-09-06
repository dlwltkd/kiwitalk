use futures_lite::{AsyncRead, AsyncWrite};
use futures_loco_protocol::{loco_protocol::command::Method, LocoClient};
use serde::{Deserialize, Deserializer, Serialize};

use crate::RequestResult;

use super::request_simple;

#[derive(Debug)]
pub struct BookingClient<T>(LocoClient<T>);

impl<T> BookingClient<T> {
    pub const fn new(client: LocoClient<T>) -> Self {
        Self(client)
    }

    pub fn into_inner(self) -> LocoClient<T> {
        self.0
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> BookingClient<T> {
    pub async fn get_conf(&mut self, req: &GetConfReq<'_>) -> RequestResult<GetConfRes> {
        request_simple(&mut self.0, Method::new("GETCONF").unwrap(), req).await
    }
}

/// Request checkin server information
#[derive(Debug, Clone, Serialize)]
pub struct GetConfReq<'a> {
    /// Network MCCMNC
    #[serde(rename = "MCCMNC")]
    pub mccmnc: &'a str,

    /// Current OS (win32, android, mac, etc.)
    pub os: &'a str,

    #[serde(rename = "userId")]
    pub user_id: i64,
}

/// Answer checkin server information
#[derive(Debug, Clone, Deserialize)]
pub struct GetConfRes {
    /// Unknown
    #[serde(default)]
    pub revision: i32,

    /// Cellular (3g) config
    #[serde(rename = "3g", deserialize_with = "deserialize_cellular_connection")]
    pub cellular: ConnectionData,

    /// WiFi, wired config
    #[serde(deserialize_with = "deserialize_wifi_connection")]
    pub wifi: ConnectionData,

    /// Contains Checkin host
    pub ticket: HostData,

    /// voice / video talk configuration(?)
    pub trailer: Trailer,

    /// voice / video talk high resolution configuration(?)
    #[serde(rename = "trailer.h")]
    pub trailer_high: TrailerHigh,

    pub etc: Etc,
}

/// ConnectionData includes ports, connection configuartion
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionData {
    /// Keep interval(?) when background
    #[serde(rename = "bgKeepItv")]
    pub background_keep_interval: i32,

    /// Reconnect interval when background
    #[serde(rename = "bgReconnItv")]
    pub background_reconnect_interval: i32,

    /// Ping interval when background
    #[serde(rename = "bgPingItv")]
    pub background_interval: i32,

    /// Ping interval
    #[serde(rename = "fgPingItv")]
    pub ping_interval: i32,

    /// Request timeout
    #[serde(default, rename = "reqTimeout")]
    pub request_timeout: i32,

    /// Encrypt type, but crate loco_protocol only supports 2 and server seems to use 2 only.
    #[serde(default = "default_encrypt_type", rename = "encType")]
    pub encrypt_type: i32,

    /// Connection timeout
    #[serde(rename = "connTimeout")]
    pub connection_timeout: i32,

    /// Header timeout
    #[serde(rename = "recvHeaderTimeout")]
    pub receive_header_timeout: i32,

    /// IN segment timeout
    #[serde(rename = "inSegTimeout")]
    pub in_seg_timeout: i32,

    /// OUT segment timeout
    #[serde(rename = "outSegTimeout")]
    pub out_seg_timeout: i32,

    /// TCP buffer size
    #[serde(rename = "blockSendBufSize")]
    pub block_send_buffer_size: i32,

    /// Port list
    pub ports: Vec<i32>,
}

/// HostData includes host list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HostData {
    /// Unknown
    pub ssl: Vec<String>,

    /// Unknown
    pub v2sl: Vec<String>,

    /// Usable host list
    pub lsl: Vec<String>,

    /// Usable host list (ipv6)
    pub lsl6: Vec<String>,
}

impl Default for HostData {
    fn default() -> Self {
        Self {
            ssl: Vec::new(),
            v2sl: Vec::new(),
            lsl: vec!["ticket-loco.kakao.com".to_owned()],
            lsl6: vec!["ticket-loco.kakao.com".to_owned()],
        }
    }
}

/// Additional config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Trailer {
    #[serde(rename = "tokenExpireTime")]
    pub token_expire_time: i32,

    pub resolution: i32,

    #[serde(rename = "resolutionHD")]
    pub resolution_hd: i32,

    #[serde(rename = "compRatio")]
    pub compress_ratio: i32,

    #[serde(rename = "compRatioHD")]
    pub compress_ratio_hd: i32,

    #[serde(rename = "downMode")]
    pub down_mode: i32,

    /// Concurrent file download limit
    #[serde(rename = "concurrentDownLimit")]
    pub concurrent_down_limit: i32,

    /// Concurrent file upload limit
    #[serde(rename = "concurrentUpLimit")]
    pub concurrent_up_limit: i32,

    #[serde(rename = "maxRelaySize")]
    pub max_relay_size: i32,

    #[serde(rename = "downCheckSize")]
    pub down_check_size: i32,

    /// Maximium attachment upload size
    #[serde(rename = "upMaxSize")]
    pub up_max_size: i32,

    #[serde(rename = "videoUpMaxSize")]
    pub video_up_max_size: i32,

    #[serde(rename = "vCodec")]
    pub video_codec: i32,

    #[serde(rename = "vFps")]
    pub video_fps: i32,

    #[serde(rename = "aCodec")]
    pub audio_codec: i32,

    /// Period that server store uploaded files
    #[serde(rename = "contentExpireTime")]
    pub content_expire_time: i32,

    #[serde(rename = "vResolution")]
    pub video_resolution: i32,

    #[serde(rename = "vBitrate")]
    pub video_bitrate: i32,

    #[serde(rename = "aFrequency")]
    pub audio_frequency: i32,

    #[serde(rename = "largeUpMaxSize")]
    pub large_up_max_size: i64,
}

impl Default for Trailer {
    fn default() -> Self {
        Self {
            token_expire_time: 1800,
            resolution: 960,
            resolution_hd: 1280,
            compress_ratio: 40,
            compress_ratio_hd: 90,
            down_mode: 0,
            concurrent_down_limit: 2,
            concurrent_up_limit: 2,
            max_relay_size: 512_000,
            down_check_size: 3_145_728,
            up_max_size: 20_971_520,
            video_up_max_size: 314_572_800,
            video_codec: 0,
            video_fps: 0,
            audio_codec: 0,
            content_expire_time: 1_209_600,
            video_resolution: 720,
            video_bitrate: 2_457_600,
            audio_frequency: 0,
            large_up_max_size: 1_073_741_824,
        }
    }
}

/// High speed trailer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrailerHigh {
    #[serde(rename = "vResolution")]
    pub video_resolution: i32,

    #[serde(rename = "vBitrate")]
    pub video_bitrate: i32,

    #[serde(rename = "aFrequency")]
    pub audio_frequency: i32,
}

impl Default for TrailerHigh {
    fn default() -> Self {
        Self {
            video_resolution: 1080,
            video_bitrate: 8_196_000,
            audio_frequency: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Etc {
    #[serde(rename = "writeRetryTimeout")]
    pub write_retry_timeout: i32,

    #[serde(rename = "wakeLockTimeout")]
    pub wake_lock_timeout: i32,

    #[serde(rename = "tracerouteHost")]
    pub traceroute_hosts: Vec<String>,

    #[serde(rename = "tracerouteHost6")]
    pub traceroute_hosts6: Vec<String>,

    #[serde(rename = "connectivityCheckUrl")]
    pub connectivity_check_url: String,
}

impl Default for Etc {
    fn default() -> Self {
        Self {
            write_retry_timeout: 7200,
            wake_lock_timeout: 10,
            traceroute_hosts: vec!["nettest.kakao.com".to_owned()],
            traceroute_hosts6: vec!["nettest.kakao.com".to_owned()],
            connectivity_check_url: String::new(),
        }
    }
}

const fn default_encrypt_type() -> i32 {
    2
}

#[derive(Debug, Deserialize, Default)]
struct ConnectionDataWire {
    #[serde(rename = "bgKeepItv")]
    background_keep_interval: Option<i32>,
    #[serde(rename = "bgReconnItv")]
    background_reconnect_interval: Option<i32>,
    #[serde(rename = "bgPingItv")]
    background_interval: Option<i32>,
    #[serde(rename = "fgPingItv")]
    ping_interval: Option<i32>,
    #[serde(rename = "reqTimeout")]
    request_timeout: Option<i32>,
    #[serde(rename = "encType")]
    encrypt_type: Option<i32>,
    #[serde(rename = "connTimeout")]
    connection_timeout: Option<i32>,
    #[serde(rename = "recvHeaderTimeout")]
    receive_header_timeout: Option<i32>,
    #[serde(rename = "inSegTimeout")]
    in_seg_timeout: Option<i32>,
    #[serde(rename = "outSegTimeout")]
    out_seg_timeout: Option<i32>,
    #[serde(rename = "blockSendBufSize")]
    block_send_buffer_size: Option<i32>,
    ports: Option<Vec<i32>>,
}

impl ConnectionDataWire {
    fn with_defaults(self, defaults: ConnectionData) -> ConnectionData {
        ConnectionData {
            background_keep_interval: self
                .background_keep_interval
                .unwrap_or(defaults.background_keep_interval),
            background_reconnect_interval: self
                .background_reconnect_interval
                .unwrap_or(defaults.background_reconnect_interval),
            background_interval: self
                .background_interval
                .unwrap_or(defaults.background_interval),
            ping_interval: self.ping_interval.unwrap_or(defaults.ping_interval),
            request_timeout: self.request_timeout.unwrap_or(defaults.request_timeout),
            encrypt_type: self.encrypt_type.unwrap_or(defaults.encrypt_type),
            connection_timeout: self
                .connection_timeout
                .unwrap_or(defaults.connection_timeout),
            receive_header_timeout: self
                .receive_header_timeout
                .unwrap_or(defaults.receive_header_timeout),
            in_seg_timeout: self.in_seg_timeout.unwrap_or(defaults.in_seg_timeout),
            out_seg_timeout: self.out_seg_timeout.unwrap_or(defaults.out_seg_timeout),
            block_send_buffer_size: self
                .block_send_buffer_size
                .unwrap_or(defaults.block_send_buffer_size),
            ports: self.ports.unwrap_or(defaults.ports),
        }
    }
}

fn default_connection_data(
    background_reconnect_interval: i32,
    background_interval: i32,
) -> ConnectionData {
    ConnectionData {
        background_keep_interval: -1,
        background_reconnect_interval,
        background_interval,
        ping_interval: 180,
        request_timeout: 0,
        encrypt_type: default_encrypt_type(),
        connection_timeout: 15,
        receive_header_timeout: 20,
        in_seg_timeout: 10,
        out_seg_timeout: 10,
        block_send_buffer_size: 2048,
        ports: vec![995, 8080, 5223, 5228, 9282, 5242, 10009],
    }
}

fn deserialize_cellular_connection<'de, D>(deserializer: D) -> Result<ConnectionData, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(ConnectionDataWire::deserialize(deserializer)?
        .with_defaults(default_connection_data(300, 1200)))
}

fn deserialize_wifi_connection<'de, D>(deserializer: D) -> Result<ConnectionData, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(ConnectionDataWire::deserialize(deserializer)?
        .with_defaults(default_connection_data(180, 600)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_get_conf_serializes_the_current_field_order_and_types() {
        let request = GetConfReq {
            mccmnc: "45008",
            os: "android",
            user_id: 42,
        };

        let document = bson::to_document(&request).unwrap();
        assert_eq!(
            document.keys().map(String::as_str).collect::<Vec<_>>(),
            ["MCCMNC", "os", "userId"]
        );
        assert_eq!(document.get_str("MCCMNC").unwrap(), "45008");
        assert_eq!(document.get_str("os").unwrap(), "android");
        assert_eq!(document.get_i64("userId").unwrap(), 42);
    }

    #[test]
    fn android_get_conf_applies_the_26_7_2_defaults_inside_sparse_sections() {
        let response: GetConfRes = bson::from_document(bson::doc! {
            "ticket": {},
            "3g": {},
            "wifi": {},
            "trailer": {},
            "trailer.h": {},
            "etc": {},
        })
        .unwrap();

        assert_eq!(response.revision, 0);
        assert_eq!(response.ticket.lsl, ["ticket-loco.kakao.com"]);
        assert_eq!(response.ticket.lsl6, ["ticket-loco.kakao.com"]);
        assert_eq!(response.cellular.background_reconnect_interval, 300);
        assert_eq!(response.cellular.background_interval, 1200);
        assert_eq!(response.wifi.background_reconnect_interval, 180);
        assert_eq!(response.wifi.background_interval, 600);
        assert_eq!(response.wifi.encrypt_type, 2);
        assert_eq!(
            response.wifi.ports,
            [995, 8080, 5223, 5228, 9282, 5242, 10009]
        );
        assert_eq!(response.trailer.large_up_max_size, 1_073_741_824);
        assert_eq!(response.trailer_high.video_resolution, 1080);
        assert_eq!(response.trailer_high.video_bitrate, 8_196_000);
        assert_eq!(response.etc.write_retry_timeout, 7200);
        assert_eq!(response.etc.traceroute_hosts, ["nettest.kakao.com"]);
    }
}
