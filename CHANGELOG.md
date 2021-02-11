# Changelog

# 0.5.0 - 2021-02-11

  - **BREAKING_CHANGE**: Update tungstenite and async-tungstenite to 0.12,

# 0.4.0 - 2021-01-01

  - **BREAKING_CHANGE**: Update tokio to v1,

# 0.4.0-beta.2 - 2020-11-23

  - **BREAKING_CHANGE**: do not enable default features on tungstenite.
  - remove thiserror.

# 0.4.0-beta.1 - 2020-11-03

  - **BREAKING_CHANGE**: update tokio to 0.3 and async-tungstenite to 0.10. Will go out of beta when tokio releases 1.0.

# 0.3.0 - 2020-10-01

  - **BREAKING_CHANGE**: update async-tungstenite to 0.8 and tungstenite to 0.11.

# 0.2.0 - 2020-06-10

  - **BREAKING_CHANGE**: update async-tungstenite to 0.5.
  - correct a documentation mistake
  - chores.

# 0.1.0 - 2020-03-21

  - **BREAKING_CHANGE**: Switch to async_tungstenite as backend, we are now framework agnostic.
  - **BREAKING_CHANGE**: Rename error type to WsErr.

  - Implement tokio `AsyncRead`/`AsyncWrite` for WsStream (Behind a feature flag).
  - delegate implementation of `AsyncRead`/`AsyncWrite`/`AsyncBufRead` to _async_io_stream_. This allows
    sharing the functionality with _ws_stream_wasm_, fleshing it out to always fill and use entire buffers,
    polling the underlying stream several times if needed.
  - only build for default target on docs.rs.
  - exclude unneeded files from package build.
  - remove trace and debug statements.

# 0.1.0-alpha.5 - 2019-11-14

  - update to futures 0.3.1.

# 0.1.0-alpha.4 - 2019-10-07

  - now handle sending out a close frame correctly when we decide to close, like when we receive a text message.

  - Non fatal errors from underlying tokio-tungstenite stream are now returned out of band. This allows to keep
    polling the stream until the close handshake is completed and it returns `None`. This is not possible for
    errors returned from `poll_read`, since codecs will no longer poll a stream as soon as it has returned an error.

# 0.1.0-alpha.2 - 2019-09-17

  - fix docs.rs readme
  - add CI testing
  - fix clippy warnings

# 0.1.0-alpha.1 - 2019-09-17 Initial release
