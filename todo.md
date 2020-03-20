# TODO

- documentation
  - make sure features are documented
- CI
- check crate template for changes.
- Look into AsyncBufReader which allows the io to allocate the buffer as opposed to AsyncRead, since tungstenite allocates a Vec for binary. This might allow avoiding  double allocation.
- changelog
- release
- put something decent in the ws_stream crate.

