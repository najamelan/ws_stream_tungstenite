# Design of ws_stream_tungstenite

## Obtaining AsyncRead/Write over a tungstenite websocket.

The WsStream::new function takes a `WebSocketStream` from _async-tungstenite_. Users are to setup their connection with _async-tungstenite_, keeping the implementation of _ws_stream_tungstenite_ free of low level details like transports and tls encryption.

Note that in order to work correctly, even though WebSocketStream takes `S: AsyncRead + AsyncWrite`, tungstenite itself will work on `S: std::io::Read + std::io::Write`, so the impls of the std traits must use `std::io::ErrorKind::WouldBlock` to indicate pending state and must wake up tasks that are waiting on this pending state.

The WsStream object implements AsyncRead/Write (futures 0.3). Currently the ecosystem has 2 versions of these traits (futures and tokio-io). We will see how the ecosystem evolves, in order to determine which version(s) to support. For the moment the version from the futures library is implemented. This can be framed with `futures-codec`. Currently there is a bug in 0.2.5 version of futures-codec, which is fixed in the master branch, so use a patch in Cargo.toml until 0.2.6 comes out.

TODO: impl AsyncRead/Write from tokio 0.2 as well.

## Obtaining information about the websocket connection.

WsStream is observable through pharos. It has an event stream wich will contain:
- errors from underlying layers (tcp, tungstenite)
- errors from ws_stream_tungstenite (we don't accept websocket text messages, only binary)
- Ping event, the remote pinged us and we responded. This contains the data from the ping.
- Close event, when we received a close frame from the remote endpoint. This contains the close frame with code and reason.

There are several reasons for out of band error notification:
- AsyncRead/Write can only return `std::io::Error`. The variants of `std::io::ErrorKind` don't always allow conveying all meaning of an underlying `tungstenite::Error`.
- A stream framed with a codec (futures-codec) will always return `None` after an error was returned. However some errors are not fatal and we need to perform a close handshake. For that to work with tokio-tungstenite we need to keep polling the stream in order to drive the close handshake to completion. This is not possible if the framed implementation returns `None` prematurely. Thus we can only return fatal errors in band.
- Since we create an AsyncRead/Write for the binary data of the connection, we cannot return websocket control frames in band. These can be close frames that contain a close code and reason which might be relevant for client code or they might want to log them.

## Closing the connection

The WsStream object takes ownership of the WebSocketSteam which in turn takes ownership of the underlying connection, so when dropping WsStream you close the underlying connection, which should only be done if:
- the websocket close handshake is complete (on the server), the client should wait for the server to close the underlying connection as defined by the websocket rfc (6455).
- a fatal error happened

Dropping the connection earlier will be considered a websocket protocol error by the remote endpoint.

When a close handshake is in progress, the only way to drive it to completion is by continuing to `poll_read` the WsStream object. In general, you can call `while let Some(msg) = stream.next().await` on a framed and split WsStream. **This means that client code should create a loop over the incoming stream and never break from it unless it returns `None` or an error or the remote endpoint does not close the connection in a timely manner**.

If you want to close the connection, call `close` on the `futures::io::WriteHalf` (will resolve immediately) and keep polling the `futures::io::ReadHalf` until it returns `None`. You might start a timer at this moment to drop the connection if the remote does not acknowledge the close handshake in a timely manner and break from your read loop to avoid hanging for too long.

When the `futures::io::ReadHalf` returns `None`, it is always safe to drop WsStream.

When you try to send data out over the `futures::io::WriteHalf` and the connection is already closed, `std::io::ErrorKind::NotConnected` will be returned and the only sensible thing to do with the `futures::io::WriteHalf` is to drop it.

When the remote endpoint initiates the close handshake, you can detect this through the event stream, tungstenite will ìmmediately schedule a close acknowledgement (and as long as you keep polling the `futures::io::ReadHalf` it will actually get sent out too). So as soon as we receive the close frame, the close handshake is considered complete and sending will return `std::io::ErrorKind::NotConnected`.

After the close handshake is complete (your endpoint has both received and sent a close frame), if you are the client, you shall be waiting for the server to close the underlying connection. `WsStream::poll_read` will not return `None` on a client until the server has closed the connection. Here too, you might want to set a timer and drop the connection if the server does not close in a timely manner.

