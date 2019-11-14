// Verify the correct error is returned when sending a text message.
//
use
{
	ws_stream_tungstenite :: { *                                                                                     } ,
	futures               :: { StreamExt, SinkExt, executor::LocalPool, task::LocalSpawnExt, compat::Sink01CompatExt } ,
	futures_codec         :: { LinesCodec, Framed                                                                    } ,
	tokio                 :: { net::{ TcpListener }                                                                  } ,
	futures::compat       :: { Future01CompatExt, Stream01CompatExt                                                  } ,
	futures_01            :: { future::{ ok, Future as _ }                                                           } ,
	tokio_tungstenite     :: { accept_async, connect_async                                                           } ,
	url                   :: { Url                                                                                   } ,
	tungstenite           :: { protocol::{ CloseFrame, frame::coding::CloseCode }                                    } ,
	pharos                :: { ObserveConfig, Observable                                                             } ,

	log           :: { * } ,
};


#[ test ]
//
fn send_text()
{
	// flexi_logger::Logger::with_str( "ping_pong=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let mut pool     = LocalPool::new();
	let     spawner  = pool.spawner();


	let server = async
	{
		let socket = TcpListener::bind( &"127.0.0.1:3016".parse().unwrap() ).unwrap();
		let mut connections = socket.incoming().compat();

		let tcp_stream = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = ok( tcp_stream ).and_then( accept_async ).compat().await.expect( "ws handshake" );
		let mut ws     = WsStream::new( s );
		let mut events = ws.observe( ObserveConfig::default() ).expect( "observe server" );

		let mut framed = Framed::new( ws, LinesCodec {} );

		let res = framed.next().await;

		assert!( res.is_none() );

		match events.next().await.expect( "protocol error" )
		{
			WsEvent::Error( e ) => assert_eq!( &ErrorKind::ReceivedText, e.kind() ),
			evt                 => assert!( false, "{:?}", evt ),
		}

		assert_eq!( None, framed.next().await.transpose().expect( "receive close stream" ) );
	};


	let client = async
	{
		let url         = Url::parse( "ws://127.0.0.1:3016" ).unwrap();
		let (socket, _) = ok( url ).and_then( connect_async ).compat().await.expect( "ws handshake" );

		let mut socket = socket.sink_compat();

		socket.send( tungstenite::Message::Text( "Hi".to_string() ) ).await.expect( "send text" );

		socket.close().await.expect( "close client end" );

		let frame = CloseFrame
		{
			code  : CloseCode::Unsupported,
			reason: "Text messages are not supported.".into(),
		};

		assert_eq!( Some( tungstenite::Message::Close( Some(frame) )), socket.next().await.transpose().expect( "close" ) );

		trace!( "drop websocket" );
	};

	spawner.spawn_local( server ).expect( "spawn server" );
	spawner.spawn_local( client ).expect( "spawn client" );

	pool.run();
}

