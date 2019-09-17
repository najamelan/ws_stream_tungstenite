//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
use
{
	ws_stream_tungstenite :: { *                                                       } ,
	futures               :: { StreamExt, AsyncReadExt, AsyncBufReadExt, io::BufReader } ,
	futures               :: { executor::LocalPool, task::LocalSpawnExt                } ,
	futures::compat       :: { Future01CompatExt, Stream01CompatExt                    } ,
	std                   :: { env, net::SocketAddr, io                                } ,
	log                   :: { *                                                       } ,
	tokio                 :: { net::{ TcpListener, TcpStream }                         } ,
	futures_01            :: { future::{ ok, Future as _ }                             } ,
	tokio_tungstenite     :: { accept_async, stream::PeerAddr                          } ,
};



fn main()
{
	// flexi_logger::Logger::with_str( "echo=trace, ws_stream_tungstenite=debug, tungstenite=warn, tokio_tungstenite=warn, tokio=warn" ).start().unwrap();

	// We only need one thread.
	//
	let mut pool     = LocalPool::new();
	let     spawner  = pool.spawner();
	let mut spawner2 = spawner.clone();


	let server = async move
	{
		let addr: SocketAddr = env::args().nth(1).unwrap_or( "127.0.0.1:3212".to_string() ).parse().unwrap();
		println!( "server task listening at: {}", &addr );

		let socket = TcpListener::bind(&addr).unwrap();
		let mut connections = socket.incoming().compat();


		while let Some( stream ) = connections.next().await
		{
			spawner.clone().spawn_local( handle_conn( stream ) ).expect( "spawn future" );
		}
	};

	spawner2.spawn_local( server ).expect( "spawn future" );
	pool.run();
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


	let s = ok( tcp_stream ).and_then( accept_async ).compat().await;

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


	info!( "Incoming connection from: {}", socket.peer_addr().expect( "peer addr" ) );

	let ws_stream = WsStream::new( socket );
	let (reader, mut writer) = ws_stream.split();

	// BufReader allows our AsyncRead to work with a bigger buffer than the default 8k.
	// This improves performance quite a bit.
	//
	match BufReader::with_capacity( 64_000, reader ).copy_buf_into( &mut writer ).await
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
