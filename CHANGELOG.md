# ws_stream_tungstenite Changelog


## [Unreleased]

  [Unreleased]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.6.1...dev


## [0.6.1]

  [0.6.1]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.6.0...0.6.1

### Updated
  - switched to asynchronous-codec from futures-codec.
  - fixed external_doc removal in rustdoc 1.54.
  - fixed assert_matches ambiguity on nightly.


## [0.6.0] - 2021-02-18

  [0.6.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.5.0...0.6.0

### Updated
  - **BREAKING_CHANGE**: Update tungstenite and async-tungstenite to 0.13
  - **BREAKING_CHANGE**: Update pharos to 0.5
  - Update async_io_stream to 0.3

## [0.5.0] - 2021-02-11

  [0.5.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.4.0...0.5.0

### Updated
  - **BREAKING_CHANGE**: Update tungstenite and async-tungstenite to 0.12

## [0.4.0] - 2021-01-01

  [0.4.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.4.0-beta.2...0.4.0

### Updated
  - **BREAKING_CHANGE**: Update tokio to v1

## [0.4.0-beta.2] - 2020-11-23

  [0.4.0-beta.2]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.4.0-beta.1...0.4.0-beta.2

### Fixed
  - **BREAKING_CHANGE**: do not enable default features on tungstenite
  - remove thiserror.


## [0.4.0-beta.1] - 2020-11-03

  [0.4.0-beta.1]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.3.0...0.4.0-beta.1

### Updated
  - **BREAKING_CHANGE**: update tokio to 0.3 and async-tungstenite to 0.10. Will go out of beta when tokio releases 1.0


## [0.3.0] - 2020-10-01

  [0.3.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.2.0...0.3.0

### Updated
  - **BREAKING_CHANGE**: update async-tungstenite to 0.8 and tungstenite to 0.11


## [0.2.0] - 2020-06-10

  [0.2.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.1.0...0.2.0

### Updated
  - **BREAKING_CHANGE**: update async-tungstenite to 0.5

### Fixed
  - correct a documentation mistake


## [0.1.0] - 2020-03-21

  [0.1.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.1.0-alpha.5...0.1.0

### Added
  - **BREAKING_CHANGE**: Switch to async_tungstenite as backend, we are now framework agnostic
  - Implement tokio `AsyncRead`/`AsyncWrite` for WsStream (Behind a feature flag).

### Fixed
  - **BREAKING_CHANGE**: Rename error type to WsErr
  - delegate implementation of `AsyncRead`/`AsyncWrite`/`AsyncBufRead` to _async_io_stream_. This allows
    sharing the functionality with _ws_stream_wasm_, fleshing it out to always fill and use entire buffers,
    polling the underlying stream several times if needed.
  - only build for default target on docs.rs.
  - exclude unneeded files from package build.
  - remove trace and debug statements.


## [0.1.0-alpha.5] - 2019-11-14

  [0.1.0-alpha.5]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.1.0-alpha.4...0.1.0-alpha.5

### Updated
  - update to futures 0.3.1.


## [0.1.0-alpha.4] - 2019-10-07

  [0.1.0-alpha.4]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.1.0-alpha.2...0.1.0-alpha.4

### Fixed
  - now handle sending out a close frame correctly when we decide to close, like when we receive a text message.
  - Non fatal errors from underlying tokio-tungstenite stream are now returned out of band. This allows to keep
    polling the stream until the close handshake is completed and it returns `None`. This is not possible for
    errors returned from `poll_read`, since codecs will no longer poll a stream as soon as it has returned an error.


## [0.1.0-alpha.2] - 2019-09-17

  [0.1.0-alpha.2]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.1.0-alpha.1...0.1.0-alpha.2

### Fixed
  - fix docs.rs readme
  - add CI testing
  - fix clippy warnings

## 0.1.0-alpha.1 - 2019-09-17 Initial release
