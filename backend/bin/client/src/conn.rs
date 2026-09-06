use std::{net::Ipv4Addr, time::Duration};

use anyhow::{bail, Context};
use kiwi_talk_api::protocol::ProtocolProfile;
use num_bigint_dig::BigUint;
use once_cell::sync::Lazy;
use talk_loco_client::{
    client::booking::{BookingClient, GetConfReq},
    client::checkin::{CheckinClient, CheckinReq, CheckinRes},
    futures_loco_protocol::{
        secure::{rsa::RsaPublicKey, LocoSecureStream},
        LocoClient,
    },
};
use tokio::{
    io::BufStream,
    net::{TcpStream, ToSocketAddrs},
    time::timeout,
};
use tokio_native_tls::{native_tls, TlsConnector};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

use crate::constants::{BOOKING_HOST, BOOKING_SERVER, TALK_NET_TYPE};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const GET_CONF_TIMEOUT: Duration = Duration::from_secs(20);
const CHECKIN_TIMEOUT: Duration = Duration::from_secs(20);
const LEGACY_LSL_ENCRYPT_TYPE: i32 = 2;

pub async fn checkin(user_id: i64, profile: ProtocolProfile) -> anyhow::Result<CheckinRes> {
    let (host, port) = discover_legacy_lsl_endpoint(profile)
        .await
        .context("failed to discover a legacy LSL ticket endpoint")?;

    let stream = create_secure_stream((host.as_str(), port))
        .await
        .with_context(|| {
            format!("failed to connect to legacy LSL ticket endpoint {host}:{port}")
        })?;

    let mut client = CheckinClient::new(LocoClient::new(stream));

    timeout(
        CHECKIN_TIMEOUT,
        client.checkin(&CheckinReq {
            user_id,
            os: profile.os(),
            app_version: profile.app_version(),
            net_type: TALK_NET_TYPE as _,
            mccmnc: profile.mccmnc(),
            language: profile.language(),
            country_iso: profile.country_iso(),
            use_sub: profile.use_sub(),
            device_name: Some(profile.model()),
        }),
    )
    .await
    .context("CHECKIN request timed out")?
    .context("CHECKIN request to legacy LSL ticket endpoint failed")
}

async fn discover_legacy_lsl_endpoint(profile: ProtocolProfile) -> anyhow::Result<(String, u16)> {
    let tcp_stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(BOOKING_SERVER))
        .await
        .context("booking TCP connection timed out")?
        .context("booking TCP connection failed")?;

    let tls_connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(false)
        .danger_accept_invalid_hostnames(false)
        .build()
        .context("failed to configure booking TLS")?;

    let tls_stream = timeout(
        TLS_HANDSHAKE_TIMEOUT,
        TlsConnector::from(tls_connector).connect(BOOKING_HOST, BufStream::new(tcp_stream)),
    )
    .await
    .context("booking TLS handshake timed out")?
    .context("booking TLS handshake or hostname verification failed")?;

    let mut client = BookingClient::new(LocoClient::new(tls_stream.compat()));
    let config = timeout(
        GET_CONF_TIMEOUT,
        client.get_conf(&GetConfReq {
            os: profile.os(),
            mccmnc: profile.mccmnc(),
            model: profile.model(),
        }),
    )
    .await
    .context("booking GETCONF request timed out")?
    .context("booking GETCONF request failed")?;

    select_legacy_lsl_endpoint(
        config.wifi.encrypt_type,
        &config.ticket.lsl,
        &config.wifi.ports,
    )
    .context("booking GETCONF response is unusable")
}

fn select_legacy_lsl_endpoint(
    encrypt_type: i32,
    hosts: &[String],
    ports: &[i32],
) -> anyhow::Result<(String, u16)> {
    if encrypt_type != LEGACY_LSL_ENCRYPT_TYPE {
        bail!(
            "unsupported Wi-Fi LSL encryption type {}; expected {LEGACY_LSL_ENCRYPT_TYPE}",
            encrypt_type
        );
    }

    let host = hosts
        .iter()
        .map(|host| host.trim())
        .find(|host| is_valid_legacy_lsl_host(host))
        .context("ticket.lsl contains no valid host")?;

    let port = ports
        .iter()
        .find_map(|port| u16::try_from(*port).ok().filter(|port| *port != 0))
        .context("wifi.ports contains no valid TCP port")?;

    Ok((host.to_owned(), port))
}

fn is_valid_legacy_lsl_host(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 || !host.is_ascii() {
        return false;
    }

    if host.parse::<Ipv4Addr>().is_ok() {
        return true;
    }

    let host = host.strip_suffix('.').unwrap_or(host);
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

static LOCO_SECURE_KEY: Lazy<RsaPublicKey> = Lazy::new(|| {
    RsaPublicKey::new(
        BigUint::from_bytes_be(&[
            0xAC, 0x58, 0x68, 0x8D, 0x45, 0x97, 0xAA, 0xEE, 0xC6, 0x46, 0x3F, 0x06, 0x58, 0xD2,
            0x20, 0x5F, 0x92, 0x7A, 0xC3, 0x6D, 0xE3, 0x6D, 0x6D, 0xEC, 0xA5, 0x8C, 0xCB, 0xBD,
            0x0A, 0x8B, 0x48, 0xA7, 0x2D, 0xE8, 0x45, 0x43, 0xE9, 0x4B, 0x7D, 0x75, 0xF5, 0xC2,
            0x03, 0xFC, 0x02, 0x13, 0xFF, 0x45, 0x7C, 0xF7, 0x89, 0x04, 0x48, 0x6A, 0xB1, 0x8E,
            0x49, 0xC5, 0x85, 0x04, 0x1D, 0x5B, 0xF3, 0xFB, 0x69, 0xBB, 0xF9, 0xC8, 0xC3, 0x0B,
            0x16, 0xD0, 0x4E, 0x14, 0x86, 0xE7, 0x3D, 0x61, 0x0D, 0x28, 0x20, 0x2F, 0x10, 0x37,
            0xD1, 0x04, 0x41, 0x17, 0x6F, 0xE0, 0x28, 0x3A, 0x3A, 0xBA, 0x6F, 0x4D, 0x83, 0x4A,
            0xB8, 0x40, 0x33, 0xBE, 0xDF, 0xE9, 0xAA, 0x57, 0x0D, 0x5C, 0x56, 0x8C, 0x74, 0x5E,
            0xDF, 0xBE, 0x47, 0x85, 0x1D, 0x35, 0x3D, 0x81, 0x91, 0x51, 0xBE, 0xBE, 0x9D, 0x13,
            0xD9, 0xCE, 0x5E, 0xD5, 0x07, 0x5E, 0x2A, 0xF6, 0x27, 0x1D, 0x82, 0x26, 0x3D, 0xB7,
            0xA8, 0x70, 0x66, 0x3E, 0x48, 0x74, 0x5C, 0x31, 0x67, 0x99, 0x3D, 0xF5, 0x2B, 0x93,
            0x46, 0xCB, 0x6E, 0x7C, 0x3A, 0x3B, 0xF2, 0x83, 0x7A, 0x2E, 0x4B, 0x39, 0x59, 0x38,
            0xDF, 0x92, 0xC4, 0xB0, 0xA1, 0xCD, 0xB6, 0x3E, 0xEE, 0xE1, 0xFB, 0x30, 0x09, 0x57,
            0x6C, 0xE0, 0x81, 0x6D, 0x82, 0x38, 0xC1, 0xAE, 0xDA, 0xEF, 0x86, 0x4E, 0x02, 0x8A,
            0xD3, 0x9C, 0x46, 0xB5, 0x55, 0xC9, 0xEB, 0x8E, 0x0C, 0x8B, 0x97, 0xCB, 0xB8, 0x37,
            0x75, 0xC6, 0xB5, 0x64, 0xB3, 0xCB, 0xC6, 0x16, 0xFA, 0x7D, 0x3D, 0xB9, 0x52, 0xD2,
            0x9D, 0xFB, 0xCF, 0xE3, 0x15, 0x32, 0x0C, 0x87, 0x89, 0xFF, 0xBA, 0x5D, 0xAE, 0xEA,
            0x98, 0xBB, 0x9F, 0x2F, 0x96, 0x94, 0x43, 0xCF, 0x78, 0x1B, 0x21, 0xC3, 0x2E, 0x22,
            0x3D, 0x0E, 0xB7, 0x3D,
        ]),
        BigUint::from(3_u8),
    )
    .unwrap()
});

pub type SecureTcpStream = LocoSecureStream<Compat<BufStream<TcpStream>>>;

pub async fn create_secure_stream<A: ToSocketAddrs>(addr: A) -> anyhow::Result<SecureTcpStream> {
    let tcp_stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(addr))
        .await
        .context("Loco TCP connection timed out")?
        .context("Loco TCP connection failed")?;

    Ok(LocoSecureStream::new(
        LOCO_SECURE_KEY.to_owned(),
        BufStream::new(tcp_stream).compat(),
    ))
}

#[cfg(test)]
mod tests {
    use super::select_legacy_lsl_endpoint;

    fn hosts(values: &[&str]) -> Vec<String> {
        values.iter().map(|host| (*host).to_owned()).collect()
    }

    #[test]
    fn selects_first_valid_legacy_lsl_host_and_port() {
        let hosts = hosts(&["", "-invalid.example", " ticket-loco.kakao.com "]);

        assert_eq!(
            select_legacy_lsl_endpoint(2, &hosts, &[-1, 0, 65_536, 995, 8080]).unwrap(),
            ("ticket-loco.kakao.com".to_owned(), 995)
        );
    }

    #[test]
    fn rejects_non_cfb_wifi_configuration() {
        let hosts = hosts(&["ticket-loco.kakao.com"]);

        let error = select_legacy_lsl_endpoint(0, &hosts, &[995]).unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported Wi-Fi LSL encryption type 0"));
    }

    #[test]
    fn rejects_configuration_without_a_valid_host() {
        let hosts = hosts(&["", "-invalid.example", "bad_name"]);

        let error = select_legacy_lsl_endpoint(2, &hosts, &[995]).unwrap_err();
        assert!(error
            .to_string()
            .contains("ticket.lsl contains no valid host"));
    }

    #[test]
    fn rejects_configuration_without_a_valid_port() {
        let hosts = hosts(&["211.183.211.10"]);

        let error = select_legacy_lsl_endpoint(2, &hosts, &[-1, 0, 65_536]).unwrap_err();
        assert!(error
            .to_string()
            .contains("wifi.ports contains no valid TCP port"));
    }
}
