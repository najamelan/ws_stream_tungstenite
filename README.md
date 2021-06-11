# ws_stream_tungstenite

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
[![Build Status](https://github.com/najamelan/ws_stream_tungstenite/workflows/ci/badge.svg?branch=master)](https://github.com/najamelan/ws_stream_tungstenite/actions)
[![Docs](https://docs.rs/ws_stream_tungstenite/badge.svg)](https://docs.rs/ws_stream_tungstenite)
[![crates.io](https://img.shields.io/crates/v/ws_stream_tungstenite.svg)](https://crates.io/crates/ws_stream_tungstenite)


> Provide an AsyncRead/Write/AsyncBufRead over websockets that can be framed with a codec.

This crate provides `AsyncRead`/`AsyncWrite`/`AsyncBufRead` over _async-tungstenite_ websockets. It mainly enables working with rust wasm code and communicating over a framed stream of bytes. This crate provides the functionality for non-WASM targets (eg. server side).
There is a WASM version [available here](https://crates.io/crates/ws_stream_wasm) for the client side.

There are currently 2 versions of the AsyncRead/Write traits. The _futures-rs_ version and the _tokio_ version. You need to enable the features `tokio_io` if you want the _tokio_ version of the traits implemented.

You might wonder, why not just serialize your struct and send it in websocket messages. First of all, on wasm there wasn't a convenient websocket rust crate before I released _ws_stream_wasm_, even without `AsyncRead`/`AsyncWrite`. Next, this allows you to keep your code generic by just taking `AsyncRead`/`AsyncWrite` instead of adapting it to a specific protocol like websockets, which is especially useful in library crates. Furthermore you don't need to deal with the quirks of a websocket protocol and library. This just works almost like any other async byte stream (exception: [closing the connection](#how-to-close-a-connection)). There is a little bit of extra overhead due to this indirection, but it should be small.

_ws_stream_tungstenite_ works on top of _async-tungstenite_, so you will have to use the API from _async-tungstenite_ to setup your
connection and pass the [`WebSocketStream`](async_tungstenite::WebSocketStream) to [`WsStream`].


## Table of Contents

- [Install](#install)
  - [Upgrade](#upgrade)
  - [Dependencies](#dependencies)
  - [Security](#security)
- [Usage](#usage)
  - [Example](#example)
  - [How to close a connection](#how-to-close-a-connection)
  - [Error Handling](#error-handling)
  - [Limitations](#limitations)
  - [API](#api)
- [References](#references)
- [Contributing](#contributing)
  - [Code of Conduct](#code-of-conduct)
- [License](#license)


## Install

With [cargo add](https://github.com/killercup/cargo-edit):
`cargo add ws_stream_tungstenite`

With [cargo yaml](https://gitlab.com/storedbox/cargo-yaml):
```yaml
dependencies:

  ws_stream_tungstenite: ^0.6
```

With raw Cargo.toml
```toml
[dependencies]

   ws_stream_tungstenite = "0.6"
```

### Upgrade

Please check out the [changelog](https://github.com/najamelan/ws_stream_tungstenite/blob/master/CHANGELOG.md) when upgrading.

### Dependencies

This crate has few dependencies. Cargo will automatically handle it's dependencies for you.

### Security

This crate uses `#![ forbid( unsafe_code ) ]`, but our dependencies don't.

Make sure your codecs have a max message size.


### Features

The `tokio_io` features enables implementing the `AsyncRead` and `AsyncWrite` traits from _tokio_.


## Usage

Please have a look in the [examples directory of the repository](https://github.com/najamelan/ws_stream_tungstenite/tree/master/examples).

The [integration tests](https://github.com/najamelan/ws_stream_tungstenite/tree/master/tests) are also useful.


### Example

This is the most basic idea (for client code):

```rust, no_run
use
{
   ws_stream_tungstenite :: { *                  } ,
   futures               :: { StreamExt          } ,
   log                   :: { *                  } ,
   async_tungstenite     :: { accept_async       } ,
   asynchronous_codec    :: { LinesCodec, Framed } ,
   async_std             :: { net::TcpListener   } ,
 };

#[ async_std::main ]
//
async fn main() -> Result<(), std::io::Error>
{
   let     socket      = TcpListener::bind( "127.0.0.1:3012" ).await?;
   let mut connections = socket.incoming();

   let tcp = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
   let s   = accept_async( tcp ).await.expect( "ws handshake" );
   let ws  = WsStream::new( s );

   // ws here is observable with pharos to detect non fatal errors and ping/close events, which cannot
   // be represented in the AsyncRead/Write API. See the events example in the repository.

   let (_sink, mut stream) = Framed::new( ws, LinesCodec {} ).split();


   while let Some( msg ) = stream.next().await
   {
      let msg = match msg
      {
         Err(e) =>
         {
            error!( "Error on server stream: {:?}", e );

            // Errors returned directly through the AsyncRead/Write API are fatal, generally an error on the underlying
            // transport.
            //
            continue;
         }

         Ok(m) => m,
      };


      info!( "server received: {}", msg.trim() );

      // ... do something useful
   }

   // safe to drop the TCP connection

   Ok(())
}
```


### How to close a connection

The websocket RFC specifies the close handshake, summarized as follows:
- when an endpoint wants to close the connection, it sends a close frame and after that it sends no more data.
  Since the other endpoint might still be sending data, it's best to continue processing incoming data, until:
- the remote sends an acknowledgment of the close frame.
- after an endpoint has both sent and received a close frame, the connection is considered closed and the server
  is to close the underlying TCP connection. The client can chose to close it if the server doesn't in a timely manner.

Properly closing the connection with _ws_stream_tungstenite_ is pretty simple. If the remote endpoint initiates the close,
just polling the stream will make sure the connection is kept until the handshake is finished. When the stream
returns `None`, you're good to drop it.

If you want to initiate the close, call close on the sink. From then on, the situation is identical to above.
Just poll the stream until it returns `None` and you're good to go.

Tungstenite will return `None` on the client only when the server closes the underlying connection, so it will
make sure you respect the websocket protocol.

If you initiate the close handshake, you might want to race a timeout and drop the connection if the remote
endpoint doesn't finish the close handshake in a timely manner. See the close.rs example in
[examples directory of the repository](https://github.com/najamelan/ws_stream_tungstenite/tree/master/examples)
for how to do that.


### Error handling

_ws_stream_tungstenite_ is about `AsyncRead`/`AsyncWrite`, so we only accept binary messages. If we receive a websocket text message,
that's considered a protocol error.

For detailed instructions, please have a look at the API docs for [`WsStream`]. Especially at the impls for `AsyncRead`/`AsyncWrite`, which detail all possible errors you can get.

Since `AsyncRead`/`AsyncWrite` only allow `std::io::Error` to be returned and on the stream some errors might not be fatal, but codecs will often consider any error to be fatal, errors are returned out of band through pharos. You should observe the `WsStream` and in the very least log any errors that are reported.


### Limitations

- No API is provided to send out Ping messages. Solving this would imply making a `WsMeta` type like
  _ws_stream_wasm_.
- Received text messages are considered an error. Another option we could consider is to return
  these to client code out of band rather than including them in the data for `AsyncRead`/`AsyncWrite`.
  This is also inconsistent with _ws_stream_wasm_ which calls `to_bytes` on them and includes the bytes
  in the bytestream.


### API

Api documentation can be found on [docs.rs](https://docs.rs/ws_stream_tungstenite).


## References

The reference documents for understanding websockets and how the browser handles them are:
- [RFC 6455 - The WebSocket Protocol](https://tools.ietf.org/html/rfc6455)
- security of ws: [WebSockets not Bound by SOP and CORS? Does this mean…](https://blog.securityevaluators.com/websockets-not-bound-by-cors-does-this-mean-2e7819374acc?gi=e4a712f5f982)
- another: [Cross-Site WebSocket Hijacking (CSWSH)](https://www.christian-schneider.net/CrossSiteWebSocketHijacking.html)


## Contributing

Please check out the [contribution guidelines](https://github.com/najamelan/ws_stream_tungstenite/blob/master/CONTRIBUTING.md).


### Testing

`cargo test --all-features`


### Code of conduct

Any of the behaviors described in [point 4 "Unacceptable Behavior" of the Citizens Code of Conduct](https://github.com/stumpsyn/policies/blob/master/citizen_code_of_conduct.md#4-unacceptable-behavior) are not welcome here and might get you banned. If anyone including maintainers and moderators of the project fail to respect these/your limits, you are entitled to call them out.

## License

[Unlicence](https://unlicense.org/)



