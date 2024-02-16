# ws_stream_tungstenite Changelog

## [Unreleased]

  [Unreleased]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.13.0...dev


## [0.13.0] - 2024-02-16

  [0.12.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.12.0...0.13.0

  - **BREAKING_CHANGE**: update async-tungstenite to 0.25


## [0.12.0] - 2024-02-09

  [0.12.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.11.0...0.12.0

  - **BREAKING_CHANGE**: update async-tungstenite to 0.24
  - **BREAKING_CHANGE**: update tungstenite to 0.21


## [0.11.0] - 2023-10-07

  [0.11.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.10.0...0.11.0
  
  - **BREAKING_CHANGE**/**SECURITY UPDATE**: update tungstenite to 0.20.1. 
    See: [RUSTSEC-2023-0065](https://rustsec.org/advisories/RUSTSEC-2023-0065).
    Make sure to check how the new version of tungstenite 
    [handles buffering](https://docs.rs/tungstenite/latest/tungstenite/protocol/struct.WebSocketConfig.html) 
    messages before sending them. Having `write_buffer_size` to anything but `0` might cause
    messages not to be sent until you flush. _ws_stream_tungstenite_ will make sure to respect
    `max_write_buffer_size`, so you shouldn't have to deal with the errors, but note that if
    you set it to something really small it might lead to performance issues on throughput.
    I wanted to roll this version out fast for the security vulnerability, but note that the 
    implementation of `AsyncWrite::poll_write_vectored` that handles compliance with `max_write_buffer_size`
    currently has no tests. If you want to use it, please review the code.     
  - **BREAKING_CHANGE**: update async-tungstenite to 0.23
  - **BREAKING_CHANGE**: switched to tracing for logging (check out the _tracing-log_ crate
    if you need to consume the events with a log consumer)


## [0.10.0]

  [0.10.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.9.0...0.10.0
  
  - **BREAKING_CHANGE**: update async-tungstenite to 0.22
  - **BREAKING_CHANGE**: update tungstenite to 0.19


## [0.9.0]

  [0.9.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.8.0...0.9.0
  
  - **BREAKING_CHANGE**: update tungstenite to 0.18


## [0.8.0]

  [0.8.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.7.0...0.8.0
  
  - **BREAKING_CHANGE**: update tungstenite to 0.17


## [0.7.0]

  [0.7.0]: https://github.com/najamelan/ws_stream_tungstenite/compare/0.6.1...0.7.0
  
  - **BREAKING_CHANGE**: update dependencies
  - In search of the cause of: https://github.com/najamelan/ws_stream_tungstenite/issues/7 error reporting
    has been improved. However until now have been unable to reproduce the issue.


## [0.6.2] 

  YANKED: has become 0.7.0 because it was a breaking change.

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
