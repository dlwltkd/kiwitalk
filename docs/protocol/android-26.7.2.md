# Android 26.7.2 protocol notes

These notes describe wire formats observed through static inspection of
KakaoTalk for Android and the corresponding KiwiTalk implementation.

The existing Android authentication code was based on
[KatokMCP's public implementation](https://github.com/mwl313/KatokMCP/blob/56ae3d40022e6431c33e382b2ff854c5b46a8a14/packages/loco-engine/src/auth/android.ts).
The observations below cover the 26.7.2 update.

## Source artifact

| Item | Value |
| --- | --- |
| Package | `com.kakao.talk` |
| Version | `26.7.2` (`29260720`) |
| Android SDK range | min 32, target 36 |
| Base APK SHA-256 | `a4d78ac71343da80954a9db69d1c6d98bb2515a7e64dce225603beeadbce08a3` |
| Signer certificate SHA-256 | `2b06cc3d47782d7c497c07f17cb5f859cd6bbcb66829f3e67b96b7a44820d2ce` |
| Extraction device | Samsung SM-S948N, Android 16 |

The phone APK contains the Android subdevice implementation, so it is useful
for request schema and constant discovery. Values obtained at runtime from the
device, especially model and Android release, still need to be checked on the
actual tablet when comparing device behavior.

Proprietary APK and decompiler output belong under `.research/`, which is
ignored by Git. Do not copy decompiled source into this repository.

## Tablet sessions

KiwiTalk uses the Android subdevice profile. The APK's tablet login flow displays
`desc_exceed_device_limit` when an existing tablet session must be logged out
before another tablet can sign in. A different model name or local device
identifier does not establish support for concurrent tablet sessions.

KiwiTalk generates and persists its own local device identifier. It does not
read the connected tablet's identifier. A tablet logout alone therefore does
not show that both clients were registered as the same device.

## HTTP authentication

The common headers are:

| Header | Value |
| --- | --- |
| `User-Agent` | `KT/26.7.2 An/<Build.VERSION.RELEASE> <language>` |
| `Accept-Language` | `<language>` |
| `A` | `android/26.7.2/<language>` |
| `X-VC` | first 16 lowercase hexadecimal characters of the SHA-512 digest |

The X-VC digest input is the UTF-8 concatenation
`BARD + userAgent + DANTE + identity + SIAN`. The app does not force
`Connection: close`; normal HTTP connection reuse applies.

Observed account paths under `android/account/`:

- `allowlist.json`
- `login.json`
- `verify_email.json`
- `request_verify_email.json`
- `passcodeLogin/generate`
- `passcodeLogin/registerDevice`
- `passcodeLogin/cancel`
- `passcodeLogin/info`
- `passcodeLogin/authorize`

`allowlist` and `login` use form encoding. Passcode generation, polling,
registration, and cancellation use JSON. The passcode generation body is:

```text
email, password, permanent,
device { name, uuid, model, osVersion }
```

`name` and `model` both come from `Build.MODEL`; `osVersion` comes from
`Build.VERSION.RELEASE`. The password is decrypted from the app's local
at-rest representation before serialization, so the wire field contains the
plain account password inside TLS.

The login form fields are:

```text
email, password, device_uuid, device_name,
auto_login?, autowithlock?, forced, permanent,
passcode?, model_name?, another_email_verification_uri?
```

An Android session refresh reuses `account/login.json` without the password:

```text
email, refresh_token, device_uuid, device_name
```

The response rotates both `access_token` and `refresh_token`. KiwiTalk stores
the refresh token only when automatic login is explicitly enabled, writes it
to a mode-0600 local file, and replaces it after every successful refresh.

## Friends, settings, and profiles

`android/friends/diff.json` is a form-encoded POST. Its current request map has
one field:

```text
friend_ids=[<id>, <id>, ...]
```

The legacy `type=a` field is no longer sent. A sparse response defaults
`total_count` to zero and `added_friends` and `deleted_ids` to empty lists.
`friendNickName`, when present, is the user's saved display name and takes
precedence over `nickName`.

`android/account/more_settings.json` now places the local account's nickname,
status, and image URLs in a nested `profile` object. KiwiTalk accepts this
layout while retaining compatibility with the older flattened response.

Current detailed profiles use a separate Pilsner service rooted at:

```text
https://talk-pilsner.kakao.com/talk/
```

Its authenticated requests use `Authorization: <access token>-<device UUID>`,
`talk-agent: android/26.7.2`, and `talk-language: <language>`. They do not use
the katalk `A` header. The relevant GET requests are:

```text
profile25/me?profileId=<id>&lastSeenAt=<token>
profile25/other?userId=<id>
```

Profile images are nested under `profileImage`, backgrounds under
`backgroundImage`, and status text under `statusMessage.message`. Image URLs
and status text can be JSON null even when their containing objects are
present.

## LOCO bootstrap

Current Android `GETCONF` sends, in order:

```text
MCCMNC, os, userId
```

It does not send the legacy `model` field. The response requires `ticket`,
`3g`, `wifi`, `trailer`, `trailer.h`, and `etc`; `revision` defaults to zero.
Missing values inside those sections use the app's built-in 26.7.2 defaults,
including distinct cellular and Wi-Fi reconnect/ping intervals, the production
ticket host, and the current media size limits.

Current Android `CHECKIN` sends:

```text
userId, os, ntype, appVer, lang, useSub?, MCCMNC?
```

`useSub` is omitted when false and `MCCMNC` is omitted when blank. The current
response parser recognizes `host`, `host6`, `port`, `cacheExpire`, `vsshost`,
`vsshost6`, and `vssport`. Legacy `cshost` fields may be absent.

Current Android `BUYCS` sends:

```text
userId, os, ntype, appVer, MCCMNC
```

Its response parser uses `vsshost`, `vsshost6`, and `vssport`, each with an
empty or zero default. Legacy call-server host fields may be absent.

Current Android `LOGINLIST` sends:

```text
appVer, prtVer, os, lang, duuid, ntype, MCCMNC,
revision, chatIds, maxIds, lastTokenId, lbk, rp, bg,
oauthToken, isSw
```

It does not send the desktop fields `dtype` or `pcst`, and it does not send
`lastChatId`. The six-byte `rp` value contains little-endian block revision,
`-1`, and plus-block revision shorts.

Subsequent Android `LCHATLIST` pages send exactly:

```text
chatIds, maxIds, lastTokenId, lastChatId
```

`lastChatId` is always present, including when it is zero. A response status of
`-310` is the protocol's partial-failure result and still carries usable page
state. Sparse responses default `eof` to true, `ltk` to `-1`, and the other
cursors and lists to zero or empty values.

The Android response state that must survive normalization is:

```text
userId, revision?, revisionInfo?, mcmRevision,
chatDatas, delChatIds, kc, eof, lastTokenId,
minLogId, pkToken?, pkUpdate, ltk, lbk,
lastChatId, flti
```

Channel list items may contain `c,t,a,n,ii,s,l,i,k,p,m,mmr,o,ll,jn,ml,li,otk,bmids`.
The `o` field is a 32-bit last-log send time. It is distinct from the 64-bit
room token returned as `o` by `CHATONROOM`. KiwiTalk retains the fields needed
for pagination, removal, unread state, preview identity, last-log ordering,
push state, and open-link identity.

The `l` value is the latest chat log, although sparse responses may include
only part of it. Its `logId` is enough to retain a list preview; missing
`chatId` and `sendAt` values can be filled from the containing room. Optional
message, attachment, author, and message-ID fields use the same defaults as
the Android parser. Discarding `l` leaves a newly synchronized room with a
server cursor but no locally renderable message.

The app treats null optional cursors and lists as their built-in defaults.
Within each item, a null or non-string `m` becomes an empty string, while
non-matching entries in the `i` and `k` display lists are ignored.

## Message writes

An ordinary Android `WRITE` contains `chatId` plus:

```text
msgId?, msg?, type, noSeen, supplement?, f?, extra?,
scope, threadId?, featureStat?, silence
```

`scope` and `silence` are always serialized. Blank `supplement`, `f`, and
`featureStat` values are omitted. A normal, non-threaded KiwiTalk text message
uses `scope = 1` (`ONLY_CHAT_ROOM`), `silence = false`, and `extra = "{}"`.

`FORWARD` has a distinct body and must not reuse the WRITE structure:

```text
chatId, msgId, type, noSeen?, extra, msg?, fromChatId?, fromLogId?
```

Secret chats use `SWRITE` and additional key material. That flow has only been
mapped statically and is not enabled by these changes.

## Room entry and history sync

Current Android `CHATONROOM` always sends `chatId` and `token`. It adds secret
chat and open-profile state only when present:

```text
st = sKeyToken?, sc = sChatToken?, opt = openlinkProfileToken?
```

Sparse room responses default `l` (last log ID), `o` (room token), member
lists, and `otk` (open-link token) to zero or empty values. The room token is
persisted and sent as `token` on the next `CHATONROOM`; it is not a read cursor.

`SYNCMSG` sends exactly:

```text
chatId, cur, max, cnt
```

Its response requires `isOK`, defaults a missing or null `chatLogs` to an empty
list, and may carry the continuation state `jsi` and `lastTokenId`.

KiwiTalk loads ordinary room history with `MCHATLOGS`, starting from that room's
saved checkpoint. Bounded `GETMSGS` lookups recover missing predecessor IDs
already referenced by records in the same room. `SYNCMSG` remains a low-level
API; it is not the ordinary history loader.

KiwiTalk keeps the latest-log target separate from its history checkpoint.
New checkpoints start at zero; cached previews and live messages do not prove
that earlier history was loaded. The repair migration resets old checkpoints
without removing messages. Each page persists its records and checkpoint in
one transaction, accepts overlapping records, and preserves deletion state.
Login also uses these checkpoints to describe locally synchronized history;
read watermarks can be ahead of this device's stored records and remain separate.

Empty pages leave the checkpoint unchanged. A terminal response that does not
reach the target, or cached records with missing predecessors, leaves history
incomplete. Retrying a completed checkpoint with gaps starts before the first
known gap. Page and time limits can be resumed from the chat view. Local
pagination uses the last database page's cursor so a sparse older cache cannot
skip records fetched during synchronization. An empty response alone does not
establish why the earlier records were unavailable.

The Android app also reads older messages from its local database. KiwiTalk can
display records returned by the server or already stored locally; it cannot
recover messages that exist only on another device. Imported text exports are
stored separately and do not change native message IDs, read markers, or history
checkpoints.

Incoming `MSG` packets may omit `noSeen`. Their nested chat logs may also omit
`msgId`, `authorId`, and `sendAt`; Android defaults these to `0`, `-1`, and `0`
respectively. Optional string values with null or another BSON type are
treated as absent.

`UPDATECHAT` always sends `chatId`; `pushAlert` and `msgTtl` are optional. The
Android client serializes a requested `msgTtl` of zero as `-1`.

## Members and trailer discovery

`GETMEM` responses require `token` and default `members` to an empty list.
`MEMBER` responses require `chatId` and also default `members` to an empty
list. Current open-chat members can omit profile image URLs and account data;
their parser uses `ptp = -1`, `pli = 0`, `opt = -1`, and
`suspicion = "UNKNOWN"` when those fields are absent.

`GETTRAILER` always sends `k` and `t`, and conditionally adds `c` and `rt`:

```text
k, t, c?, rt?
```

Its response requires `vh` and `p`, while `vh6` is optional and `rd` defaults
to false.

## Media transfer

The current Android media commands serialize these fields in order:

```text
POST:  u,k,t,s,c,mid,w,h,mm,nt,os,av,ex,f,sp,ns,dt,scp,tid?,featureStat?,silence
MPOST: u,k,t,s,mm,nt,os,av,dt,scp,tid?
DOWN:  u,k,o,mm,nt,os,av,c,rt
MINI:  u,k,o,mm,nt,os,av,c
```

`POST` writes BSON null for absent `f` and `sp`; blank `featureStat` is omitted.
`MINI` has no width or height fields. Upload offset `o` and download size `s`
are BSON 32-bit integers and are widened internally after parsing.

## Read acknowledgement and room exit

`NOTIREAD` is absent from the 26.7.2 command enum. Room entry uses
`CHATONROOM`, and an incoming real-time `MSG` is answered on the same packet ID
with a `MSG` body containing `notiRead`. KiwiTalk no longer sends the obsolete
standalone `NOTIREAD` request. The transport supports this same-packet response;
KiwiTalk also advances its local read watermark when the room is active or
`CHATONROOM` reports that notification reads should be applied.

`LEAVE` always sends `chatId` and `f`. The latter has the form
`<room tracker>|<Unix milliseconds>|<reason>`. `block` is included only when
true. `report` and `li` are included together for a report. The current Android
serializer represents enabled `silence` with the numeric chat ID.

The open-profile update push is named `SYNCLINKPF`. Its body requires `olu` and
`li`, with optional room ID `c`. The embedded open-link user uses a 32-bit
`opt`, a 64-bit `pv`, and the same missing-value defaults as the Android parser.

## Validation boundary

Automated tests serialize requests to BSON or capture them with a loopback HTTP
server. They parse representative local responses. They never use stored
credentials, open the KakaoTalk app, or connect to Kakao services.

Manual Linux validation covered login, channel previews, text messaging, and
loading room history. This does not establish full history recovery or complete
media and secret-chat support.
