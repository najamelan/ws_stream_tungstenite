# ws_stream_tungstenite

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
[![Build Status](https://api.travis-ci.org/najamelan/ws_stream_tungstenite.svg?branch=master)](https://travis-ci.org/najamelan/ws_stream_tungstenite)
[![Docs](https://docs.rs/ws_stream_tungstenite/badge.svg)](https://docs.rs/ws_stream_tungstenite)
[![crates.io](https://img.shields.io/crates/v/ws_stream_tungstenite.svg)](https://crates.io/crates/ws_stream_tungstenite)


> Provide an AsyncRead/Write over websockets that can be framed with a codec.

This crate provides AsyncRead/Write over _async-tungstenite_ websockets. It mainly enables working with rust wasm code and communicating over a framed stream of bytes. This crate provides the functionality for non-WASM targets (eg. server side).
There is a WASM version [available here](https://crates.io/crates/ws_stream_wasm) for the client side.

There are currently 2 versions of the AsyncRead/Write traits. The _futures-rs_ version and the _tokio_ version. This crate implements the _futures-rs_ version only for now. We will see how the ecosystem evolves and adapt. This means you can frame your connection with the [`futures-codec`](https://crates.io/crates/futures_codec) crate. You can send arbitrary rust structs using [`futures_cbor_codec`](https://crates.io/crates/futures_cbor_codec). Know that the interface of _futures-codec_ is identical to the _tokio-codec_ one, so converting a codec is trivial.

You might wonder, why not just serialize your struct and send it in websocket messages. First of all, on wasm there wasn't a convenient websocket rust crate before I released _ws_stream_wasm_, even without AsyncRead/Write. Next, this allows you to keep your code generic by just taking AsyncRead/Write instead of adapting it to a specific protocol like websockets, which is especially useful in library crates. Furthermore you don't need to deal with the quirks of a websocket protocol and library. This just works almost like any other async byte stream (exception: [closing the connection](#how-to-close-a-connection)). There is a little bit of extra overhead due to this indirection, but it should be small.

_ws_stream_tungstenite_ works on top of _async-tungstenite_, so you will have to use the API from _async-tungstenite_ to setup your
connection and pass the [`WebSocketStream`](async_tungstenite::WebSocketStream) to [`WsStream`].

## Table of Contents

- [Install](#install)
  - [Upgrade](#upgrade)
  - [Dependencies](#dependencies)
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

  ws_stream_tungstenite: ^0.1
```

With raw Cargo.toml
```toml
[dependencies]

   ws_stream_tungstenite = "0.1"
```

### Upgrade

Please check out the [changelog](https://github.com/najamelan/ws_stream_tungstenite/blob/master/CHANGELOG.md) when upgrading.

### Dependencies

This crate has few dependencies. Cargo will automatically handle it's dependencies for you.

```yaml
dependencies:

  # public deps. Bump major version if you change their version number here.
  #
  futures           : { version: ^0.3, default-features: false }
  log               : ^0.4
  tungstenite       : ^0.10
  pharos            : ^0.4
  async-tungstenite : ^0.4
  tokio             : { version: ^0.2, optional: true }

  # private deps
  #
  pin-utils         : ^0.1.0-alpha
  bitflags          : ^1
```

### Features

There are no optional features.

## Usage

Please have a look in the [examples directory of the repository](https://github.com/najamelan/ws_stream_tungstenite/tree/master/examples).

The [integration tests](https://github.com/najamelan/ws_stream_tungstenite/tree/master/tests) are also useful.

### Example

This is the most basic idea (for client code):

```rust
use
{
   ws_stream_tungstenite :: { *                                    } ,
   futures               :: { StreamExt                            } ,
   futures::compat       :: { Future01CompatExt, Stream01CompatExt } ,
   log                   :: { *                                    } ,
   tokio                 :: { net::{ TcpListener }                 } ,
   futures_01            :: { future::{ ok, Future as _ }          } ,
   tokio_tungstenite     :: { accept_async                         } ,
   futures_codec         :: { LinesCodec, Framed                   } ,
};

async fn run()
{
   let     socket      = TcpListener::bind( &"127.0.0.1:3012".parse().unwrap() ).unwrap();
   let mut connections = socket.incoming().compat();

   let tcp = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
   let s   = ok( tcp ).and_then( accept_async ).compat().await.expect( "ws handshake" );
   let ws  = WsStream::new( s );

   // ws here is observable with pharos to detect non fatal errors and ping/close events, which cannot
   // be represented in the AsyncRead/Write API. See the events example in the repository.

   let (mut sink, mut stream) = Framed::new( ws, LinesCodec {} ).split();


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

_ws_stream_tungstenite_ is about AsyncRead/Write, so we only accept binary messages. If we receive a websocket text message,
that's considered a protocol error.

For detailed instructions, please have a look at the API docs for [`WsStream`]. Especially at the impls for AsyncRead/Write, which detail all possible errors you can get.


### Limitations

- no convenient support for closing with reason and code.


### API

Api documentation can be found on [docs.rs](https://docs.rs/ws_stream_tungstenite).


## References

The reference documents for understanding websockets and how the browser handles them are:
- [RFC 6455 - The WebSocket Protocol](https://tools.ietf.org/html/rfc6455)
- security of ws: https://blog.securityevaluators.com/websockets-not-bound-by-cors-does-this-mean-2e7819374acc?gi=e4a712f5f982
- another: https://www.christian-schneider.net/CrossSiteWebSocketHijacking.html


## Contributing

Please check out the [contribution guidelines](https://github.com/najamelan/ws_stream_tungstenite/blob/master/CONTRIBUTING.md).



### Testing

`cargo test`


### Code of conduct

Any of the behaviors described in [point 4 "Unacceptable Behavior" of the Citizens Code of Conduct](http://citizencodeofconduct.org/#unacceptable-behavior) are not welcome here and might get you banned. If anyone including maintainers and moderators of the project fail to respect these/your limits, you are entitled to call them out.

## License

[Unlicence](https://unlicense.org/)



