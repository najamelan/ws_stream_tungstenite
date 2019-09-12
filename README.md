# ws_stream_tungstenite

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
[![Build Status](https://api.travis-ci.org/najamelan/ws_stream_tungstenite.svg?branch=master)](https://travis-ci.org/najamelan/ws_stream_tungstenite)
[![Docs](https://docs.rs/ws_stream_tungstenite/badge.svg)](https://docs.rs/ws_stream_tungstenite)
[![crates.io](https://img.shields.io/crates/v/ws_stream_tungstenite.svg)](https://crates.io/crates/ws_stream_tungstenite)


> Provide an AsyncRead/Write over websockets that can be framed with a codec.

This crate provides AsyncRead/Write over websockets. It mainly enables working with rust wasm code and communicating over a framed stream of bytes. This crate provides the functionality for non-WASM targets. There is a WASM version [available here](https://crates.io/crates/ws_stream_tungstenite_wasm).

There are currently 2 versions of the AsyncRead/Write traits. The futures-rs version and the tokio version. This crate implements the futures version only for now. We will see how the ecosystem evolves and adapt. This means you can frame your connection with the [`futures-codec`](https://crates.io/crates/futures_codec) crate. You can send arbitrary rust structs using [`futures_cbor_codec`](https://crates.io/crates/futures_cbor_codec). Know that the interface of futures-codec is identical to the tokio-codec one, so converting a codec is trivial.

You might wonder, why not just serialize your struct and send it in websocket messages. First of all, on wasm there wasn't a convennient rust crate before I released ws_stream_tungstenite_wasm. Next, this allows you to keep your code generic by just taking AsyncRead/Write instead of adapting it to a specific protocol like websockets, which is especially useful in library crates.

Currently backend providers are still on futures 0.1, so frequent changes are expected in the upcoming months. We support tokio-tunstenite and warp. Warp support has been included because often when writing a fullstack rust application, even if you prefer communicating over websockets, you will usually need to serve some static files as well. Warp adds a rather convenient http server with good performance for your static files.

ws_stream_tungstenite provides both client and server functionality over tokio-tungstenite, and server functionality with warp. Both tls and plain connections are supported for each of those.

## Table of Contents

- [Install](#install)
  - [Upgrade](#upgrade)
  - [Dependencies](#dependencies)
- [Usage](#usage)
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

   ws_stream_tungstenite = "^0.1"
```

### Upgrade

Please check out the [changelog](https://github.com/najamelan/ws_stream_tungstenite/blob/master/CHANGELOG.md) when upgrading.

### Dependencies

This crate has few dependiencies. Cargo will automatically handle it's dependencies for you.

### Features

Either backend can be enabled through the features `warp` and `tungstenite`.

TLS support needs to be enabled with the `tls` feature.


## Usage

Please have a look in the [examples directory of the repository](https://github.com/najamelan/ws_stream_tungstenite/tree/master/examples).

The [integration tests](https://github.com/najamelan/ws_stream_tungstenite/tree/master/tests) are also useful.


### How to close a connection
The websocket RFC specefies the close handshake, summarized as follows:
- when an endpoint wants to close the connection, it sends an close frame and after that it sends no more data.
  Since the other endpoint might still be sending data, it's best to continue processing incoming data, until:
- the remote sends an acknowledgement of the close frame.
- after an endpoint has both sent and received a close frame, the connection is considered closed and the server
  is to close the underlying tcp connection. The client can chose to close it if the server doesn't in a timely manner.

Let's see how we can do this with ws_stream_tungstenite:
```rust
```


### Scenarios


#### tokio-tungstenite server on TCP

**Note**: tokio-tungstenite is currently on still on futures 0.1.

You can use [TungWebSocket::listen](crate::providers::TungWebSocket::listen) if you just want TCP.

Please have a look at the [echo example](https://github.com/najamelan/ws_stream_tungstenite/tree/master/examples/echo.rs) for example code.

Listen returns an [Incoming] which yields [Handshake] which in turn will resolve when the Websocket handshake is complete.

#### tokio-tungstenite server on other streams

You can listen for incoming connections yourself and create a [TungWebSocket](crate::providers::TungWebSocket) over any stream that implements AsyncRead01/AsyncWrite01.

TODO: example

#### tokio-tungstenite server over tls


#### warp server

#### warp server over https

#### tokio-tungstenite client

#### tokio-tungstenite client over tls


### Closing the connection

Neither tokio-tungstenite nor warp support closing with a reason and close code. The way to implement it would be to create a tungstenite Message and send that, but that won't work on warp. Thus, I haven't bothered to implement this functionality yet.

Closing without reason and code is as simple as calling close on the Sink.

Note that when you use stream combinators, eg. to make an echo server:
```rust
match stream.forward( sink ).await
{
	...
}
```
This will return an error when the client closes the connection. The forward combinator will find the stream returning None, and will thus try to close the Sink, but since that's the same connection, it's already closed. This will return `std::io::ErrorKind::NotConnected`.

### Error handling



### Limitations

- no convenient support for closing with reason and code (see above)
- only tokio-tungstenite and warp backends supported for now


## API

Api documentation can be found on [docs.rs](https://docs.rs/ws_stream_tungstenite).


## References
The reference documents for understanding websockets and how the browser handles them are:
- [HTML Living Standard](https://html.spec.whatwg.org/multipage/web-sockets.html)
- [RFC 6455 - The WebSocket Protocol](https://tools.ietf.org/html/rfc6455)
- security of ws: https://blog.securityevaluators.com/websockets-not-bound-by-cors-does-this-mean-2e7819374acc?gi=e4a712f5f982
- another: https://www.christian-schneider.net/CrossSiteWebSocketHijacking.html

## Contributing

This repository accepts contributions. Ideas, questions, feature requests and bug reports can be filed through github issues.

Pull Requests are welcome on github. By commiting pull requests, you accept that your code might be modified and reformatted to fit the project coding style or to improve the implementation. Please discuss what you want to see modified before filing a pull request if you don't want to be doing work that might be rejected.

Please file PR's against the `dev` branch, don't forget to update the changelog and the documentation.

### Testing

tests/certs has an https certificate for the domain ws.stream. This is required for the ssl integration tests. After running the tests, remove the certificates from your system.

#### Arch linux
 In arch linux it can be installed to work like:
```shell
sudo cp tests/certs/ws.stream.crt /etc/ca-certificates/trust-source/anchors/
trust extract-compat
```
#### Debian/derivatives
As root:
```shell
apt install ca-certificates
cp tests/certs/ws.stream.crt /usr/local/share/ca-certificates/
update-ca-certificates
```

`cargo test --all-features`

### Code of conduct

Any of the behaviors described in [point 4 "Unacceptable Behavior" of the Citizens Code of Conduct](http://citizencodeofconduct.org/#unacceptable-behavior) are not welcome here and might get you banned. If anyone including maintainers and moderators of the project fail to respect these/your limits, you are entitled to call them out.

## License

[Unlicence](https://unlicense.org/)



