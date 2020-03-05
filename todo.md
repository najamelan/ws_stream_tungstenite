# TODO

- get rid of the ok().and_then
- test with async-std
- Look into AsyncBufReader which allows the io to allocate the buffer as opposed to AsyncRead, since tungstenite allocates a Vec for binary. This might allow avoiding  double allocation.
