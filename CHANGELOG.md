# Changelog

# 0.1.0-alpha.3 - 2019-10-06

  - now handle sending out a close frame correctly when we decide to close, like when we receive a text message.

  - Non fatal errors from underlying tokio-tungstenite stream are now returned out of band. This allows to keep
    polling the stream until the close handshake is completed and it returns `None`. This is not possible for
    errors returned from `poll_read`, since codecs will no longer poll a stream as soon as it has returned an error.

# 0.1.0-alpha.2 - 2019-09-17

  - fix docs.rs readme
  - add CI testing
  - fix clippy warnings

# 0.1.0-alpha.1 - 2019-09-17 Initial release
