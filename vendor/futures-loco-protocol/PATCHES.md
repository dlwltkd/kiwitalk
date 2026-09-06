# Local changes to futures-loco-protocol 0.4.0

Source: the crates.io `futures-loco-protocol` 0.4.0 release, maintained at
<https://github.com/storycraft/futures-loco-protocol/>. The upstream MIT license
is preserved in [LICENSE](./LICENSE).

`LocoClient::write_command` and `LocoSession::respond` let a reply preserve the
peer's packet header without allocating a request ID or awaiting another reply.
KiwiTalk uses this for incoming message acknowledgements. A local transport test
checks the serialized header and body.

The manifest adds `futures-lite` for that test and points to `README.md` with
matching filename case. The README uses LF line endings, and Rust imports are
formatted with rustfmt.

The workspace selects this copy through `[patch.crates-io]` in `Cargo.toml`.
