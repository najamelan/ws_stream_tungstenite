//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
use
{
	ws_stream_tungstenite :: { *                                                    } ,
	futures               :: { StreamExt, AsyncReadExt, io::{ BufReader, copy_buf } } ,
	std                   :: { env, net::SocketAddr, io                             } ,
	log                   :: { *                                                    } ,
	tokio                 :: { net::{ TcpListener, TcpStream }                      } ,
	async_tungstenite     :: { accept_async, tokio::{ TokioAdapter }                } ,
};


#[tokio::main]
//
async fn main()
{
	// flexi_logger::Logger::with_str( "echo=trace, ws_stream_tungstenite=debug, tungstenite=warn, tokio_tungstenite=warn, tokio=warn" ).start().unwrap();

	let addr: SocketAddr = env::args().nth(1).unwrap_or( "127.0.0.1:3212".to_string() ).parse().unwrap();
	println!( "server task listening at: {}", &addr );

	let mut socket      = TcpListener::bind(&addr).await.unwrap();
	let mut connections = socket.incoming();


	while let Some( stream ) = connections.next().await
	{
		tokio::spawn( handle_conn( stream ) );
	}
}


async fn handle_conn( stream: Result< TcpStream, io::Error> )
{

	// If the TCP stream fails, we stop processing this connection
	//
	let tcp_stream = match stream
	{
		Ok(tcp) => tcp,

		Err(e) =>
		{
			debug!( "Failed TCP incoming connection: {}", e );
			return;
		}
	};

	let peer_addr = tcp_stream.peer_addr();
	let s = accept_async( TokioAdapter(tcp_stream) ).await;

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


	info!( "Incoming connection from: {}", peer_addr.expect( "peer addr" ) );

	let ws_stream = WsStream::new( socket );
	let (reader, mut writer) = ws_stream.split();

	// BufReader allows our AsyncRead to work with a bigger buffer than the default 8k.
	// This improves performance quite a bit.
	//
	match copy_buf( BufReader::with_capacity( 64_000, reader ), &mut writer ).await
	{
		Ok(_) => {},

		Err(e) => match e.kind()
		{
			// Other errors we want to know about
			//
			_ => { error!( "{:?}", e.kind() ) }
		}
	}
}
