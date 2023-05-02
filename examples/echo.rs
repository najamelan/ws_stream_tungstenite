//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
use
{
	ws_stream_tungstenite :: { *                                         } ,
	futures               :: { AsyncReadExt, io::{ BufReader, copy_buf } } ,
	std                   :: { env, net::SocketAddr, io                  } ,
	log                   :: { *                                         } ,
	tokio                 :: { net::{ TcpListener, TcpStream }           } ,
	async_tungstenite     :: { accept_async, tokio::{ TokioAdapter }     } ,
};


#[tokio::main]
//
async fn main()
{
	// flexi_logger::Logger::with_str( "echo=trace, ws_stream_tungstenite=debug, tungstenite=warn, tokio_tungstenite=warn, tokio=warn" ).start().unwrap();

	let addr: SocketAddr = env::args().nth(1).unwrap_or_else( || "127.0.0.1:3212".to_string() ).parse().unwrap();
	println!( "server task listening at: {}", &addr );

	let socket = TcpListener::bind(&addr).await.unwrap();

	loop
	{
		tokio::spawn( handle_conn( socket.accept().await ) );
	}
}


async fn handle_conn( stream: Result< (TcpStream, SocketAddr), io::Error> )
{
	// If the TCP stream fails, we stop processing this connection
	//
	let (tcp_stream, peer_addr) = match stream
	{
		Ok( tuple ) => tuple,

		Err(e) =>
		{
			debug!( "Failed TCP incoming connection: {}", e );
			return;
		}
	};

	let s = accept_async( TokioAdapter::new(tcp_stream) ).await;

	// If the Ws handshake fails, we stop processing this connection
	//
	let socket = match s
	{
		Ok(ws) => ws,

		Err(e) =>
		{
			debug!( "Failed WebSocket HandShake: {}", e );
			return;
		}
	};


	info!( "Incoming connection from: {}", peer_addr );

	let ws_stream = WsStream::new( socket );
	let (reader, mut writer) = ws_stream.split();

	// BufReader allows our AsyncRead to work with a bigger buffer than the default 8k.
	// This improves performance quite a bit.
	//
	if let Err(e) = copy_buf( BufReader::with_capacity( 64_000, reader ), &mut writer ).await
	{
		error!( "{:?}", e.kind() )
	}
}
