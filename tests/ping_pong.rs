// Verify whether we handle ping and pong frames correctly
// Note that they can hold data, but as we provide AsyncRead/AsyncWrite, we generally don't
// want to produce any data on ping/pong. Just on binary messages.
// This is to check whether tokio-tungstenite will swallow them, or if we need to handle them.
//
// The answer is yes! we have to handle them. Still found an issue with tungstenite thanks to this
// test, PR filed!
//
use
{
	ws_stream_tungstenite :: { *                                                                              } ,
	futures               :: { StreamExt, SinkExt, executor::LocalPool, task::LocalSpawnExt, compat::Sink01CompatExt } ,
	futures_codec         :: { LinesCodec, Framed                                                             } ,
	tokio                 :: { net::{ TcpListener }                                                           } ,
	futures::compat       :: { Future01CompatExt, Stream01CompatExt                                           } ,
	futures_01            :: { future::{ ok, Future as _ }                                                    } ,
	tokio_tungstenite     :: { accept_async, connect_async                                                    } ,
	url                   :: { Url                                                                            } ,

	log           :: { * } ,
};


#[ test ]
//
fn ping_pong()
{
	// flexi_logger::Logger::with_str( "ping_pong=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let mut pool     = LocalPool::new();
	let     spawner  = pool.spawner();


	let server = async
	{
		let socket = TcpListener::bind( &"127.0.0.1:3015".parse().unwrap() ).unwrap();
		let mut connections = socket.incoming().compat();

		let tcp_stream = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = ok( tcp_stream ).and_then( accept_async ).compat().await.expect( "ws handshake" );
		let ws         = WsStream::new( s );

		let mut framed = Framed::new( ws, LinesCodec {} );

		assert_eq!( None, framed.next().await.transpose().expect( "receive on framed" ) );
	};


	let client = async
	{
		let url         = Url::parse( "ws://127.0.0.1:3015" ).unwrap();
		let (socket, _) = ok( url ).and_then( connect_async ).compat().await.expect( "ws handshake" );

		let mut socket = socket.sink_compat();

		socket.send( tungstenite::Message::Ping( vec![1, 2, 3] ) ).await.expect( "send ping" );

		socket.close().await.expect( "close client end" );

		assert_eq!( Some( tungstenite::Message::Pong( vec![1, 2, 3] ) ), socket.next().await.transpose().expect( "pong"  ) );
		assert_eq!( Some( tungstenite::Message::Close(None)           ), socket.next().await.transpose().expect( "close" ) );

		trace!( "drop websocket" );
	};

	spawner.spawn_local( server ).expect( "spawn server" );
	spawner.spawn_local( client ).expect( "spawn client" );

	pool.run();
}

