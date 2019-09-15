// Test using the AsyncRead/AsyncWrite from futures 0.3
//
// ✔ frame with futures-codec
// ✔ send/receive half a frame
//


use
{
	ws_stream_tungstenite :: { *                                                                       } ,
	futures       :: { StreamExt, SinkExt, channel::oneshot, executor::LocalPool, task::LocalSpawnExt  } ,
	futures_codec :: { LinesCodec, Framed                                                              } ,
	tokio                 :: { net::{ TcpListener }                                                    } ,
	futures::compat       :: { Future01CompatExt, Stream01CompatExt                                    } ,
	futures_01            :: { future::{ ok, Future as _ }                                             } ,
	tokio_tungstenite     :: { accept_async, connect_async                                             } ,
	url                   :: { Url                                                                     } ,

	// log           :: { * } ,
};


#[ test ]
//
fn frame03()
{
	// flexi_logger::Logger::with_str( "futures_codec=trace, ws_stream=trace, tokio=warn" ).start().expect( "flexi_logger");

	let mut pool     = LocalPool::new();
	let mut spawner  = pool.spawner();


	let server = async
	{
		let socket = TcpListener::bind( &"127.0.0.1:3012".parse().unwrap() ).unwrap();
		let mut connections = socket.incoming().compat();

		let tcp_stream = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = ok( tcp_stream ).and_then( accept_async ).compat().await.expect( "ws handshake" );
		let server     = WsStream::new( s );

		let (mut sink, mut stream) = Framed::new( server, LinesCodec {} ).split();

		sink.send( "A line\n"       .to_string() ).await.expect( "Send a line" );
		sink.send( "A second line\n".to_string() ).await.expect( "Send a line" );

		sink.close().await.expect( "close server" );

		let read = stream.next().await.transpose().expect( "close connection" );

		assert_eq!( None, read );
	};


	let client = async
	{
		let url    = Url::parse( "ws://127.0.0.1:3012" ).unwrap();
		let socket = ok( url ).and_then( connect_async ).compat().await.expect( "ws handshake" );

		let     client = WsStream::new( socket.0 );
		let mut framed = Framed::new( client, LinesCodec {} );


		let res = framed.next().await.expect( "Receive some" ).expect( "Receive a line" );
		assert_eq!( "A line\n".to_string(), res );


		let res = framed.next().await.expect( "Receive some" ).expect( "Receive a second line" );
		assert_eq!( "A second line\n".to_string(), res );


		let res = framed.next().await;
		dbg!( &res );
		assert!( res.is_none() );
	};

	spawner.spawn_local( server ).expect( "spawn server" );
	spawner.spawn_local( client ).expect( "spawn client" );

	pool.run();
}


// Receive half a frame
//
#[ test ]
//
fn partial()
{
	// flexi_logger::Logger::with_str( "futures_codec=trace, tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let mut pool     = LocalPool::new();
	let mut spawner  = pool.spawner();

	let (tx, rx) = oneshot::channel();

	let server = async move
	{
		let socket = TcpListener::bind( &"127.0.0.1:3013".parse().unwrap() ).unwrap();
		let mut connections = socket.incoming().compat();

		let tcp_stream = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = ok( tcp_stream ).and_then( accept_async ).compat().await.expect( "ws handshake" );
		let server     = WsStream::new( s );


		let (mut sink, mut stream) = Framed::new( server, LinesCodec {} ).split();

		sink.send( "A ".to_string() ).await.expect( "Send a line" );

		// Make sure the client tries to read on a partial line first.
		//
		rx.await.expect( "read channel" );

		sink.send( "line\n"         .to_string() ).await.expect( "Send a line" );
		sink.send( "A second line\n".to_string() ).await.expect( "Send a line" );

		sink.close().await.expect( "close server" );

		let read = stream.next().await.transpose().expect( "close connection" );

		assert_eq!( None, read );
	};


	let client = async move
	{
		let url    = Url::parse( "ws://127.0.0.1:3013" ).unwrap();
		let socket = ok( url ).and_then( connect_async ).compat().await.expect( "ws handshake" );

		let     client = WsStream::new( socket.0 );
		let mut framed = Framed::new( client, LinesCodec {} );

		// This will not return pending, so we will call framed.next() before the server task will send
		// the rest of the line.
		//
		tx.send(()).expect( "trigger channel" );
		let res = framed.next().await.expect( "Receive some" ).expect( "Receive a line" );
		assert_eq!( "A line\n".to_string(), dbg!( res ) );


		let res = framed.next().await.expect( "Receive some" ).expect( "Receive a second line" );
		assert_eq!( "A second line\n".to_string(), dbg!( res ) );


		let res = framed.next().await;
		assert!( dbg!( res ).is_none() );
	};

	spawner.spawn_local( server ).expect( "spawn server" );
	spawner.spawn_local( client ).expect( "spawn client" );

	pool.run();
}
