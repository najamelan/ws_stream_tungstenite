//! An echo server using just tokio tungstenite. This allows comparing the
//! performance overhead of ws_stream and allows testing ws_stream_wasm for text
//! messages as ws_stream only does binary.
//
use
{
	futures           :: { StreamExt                         } ,
	async_tungstenite :: { accept_async, tokio::TokioAdapter } ,
	tokio             :: { net::{ TcpListener, TcpStream }   } ,
	log               :: { *                                 } ,
	std               :: { env, net::SocketAddr              } ,
};


#[tokio::main]
//
async fn main()
{
	// flexi_logger::Logger::with_str( "echo_tt=trace, tokio=trace, tungstenite=trace, tokio_tungstenite=trace" ).start().unwrap();

	let addr: SocketAddr = env::args().nth(1).unwrap_or_else( || "127.0.0.1:3212".to_string() ).parse().unwrap();
	let socket = TcpListener::bind( addr ).await.unwrap();

	println!( "Listening on: {}", addr );


	loop
	{
		tokio::spawn( handle_conn( socket.accept().await ) );
	}
}


async fn handle_conn( conn: Result< (TcpStream, SocketAddr), std::io::Error > )
{
	// If the TCP stream fails, we stop processing this connection
	//
	let (tcp_stream, peer_addr) = match conn
	{
		Ok(tuple) => tuple,
		Err(_) =>
		{
			debug!( "Failed TCP incoming connection" );
			return;
		}
	};


	let handshake = accept_async( TokioAdapter::new(tcp_stream) );


	// If the Ws handshake fails, we stop processing this connection
	//
	let ttung = match handshake.await
	{
		Ok( ws ) => ws,

		Err(_) =>
		{
			debug!( "Failed WebSocket HandShake" );
			return;
		}
	};

	let (sink, stream) = ttung.split();

	println!( "New WebSocket connection: {}", peer_addr );


	match stream.forward( sink ).await
	{
		Ok(()) => {},

		Err(e) => match e
		{
			// When the client closes the connection, the stream will return None, but then
			// `forward` will call poll_close on the sink, which obviously is the same connection,
			// and thus already closed. Thus we will always get a ConnectionClosed error at the end of
			// this, so we ignore it.
			//
			// In principle this risks missing the error if it happens before the connection is
			// supposed to end, so in production code you should probably manually implement forward
			// for an echo server.
			//
			tungstenite::error::Error::ConnectionClosed |
			tungstenite::error::Error::AlreadyClosed    => {}

			// Other errors we want to know about
			//
			_ => { panic!( e ) }
		}
	}
}
