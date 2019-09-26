// Verify the correct error is returned when sending a protocol error.
//
use
{
	ws_stream_tungstenite :: { *                                                                              } ,
	futures               :: { StreamExt, SinkExt, executor::block_on, future::join, compat::Sink01CompatExt  } ,
	futures_codec         :: { LinesCodec, Framed                                                             } ,
	tokio                 :: { net::{ TcpListener }                                                           } ,
	futures::compat       :: { Future01CompatExt, Stream01CompatExt                                           } ,
	futures_01            :: { future::{ ok, Future as _ }                                                    } ,
	tokio_tungstenite     :: { accept_async, connect_async                                                    } ,
	url                   :: { Url                                                                            } ,
	tungstenite           :: { protocol::{ CloseFrame, frame::coding::CloseCode }                             } ,
	pharos                :: { Observable, ObserveConfig                                                      } ,

	log           :: { * } ,
};


#[ test ]
//
fn protocol_error()
{
	// flexi_logger::Logger::with_str( "ping_pong=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");


	let server = async
	{
		let socket = TcpListener::bind( &"127.0.0.1:3016".parse().unwrap() ).unwrap();
		let mut connections = socket.incoming().compat();

		let tcp_stream = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = ok( tcp_stream ).and_then( accept_async ).compat().await.expect( "ws handshake" );
		let mut ws     = WsStream::new( s );
		let mut events = ws.observe( ObserveConfig::default() ).expect( "observe" );
		let mut framed = Framed::new( ws, LinesCodec {} );


		assert!( framed.next().await.is_none() );

		match events.next().await.expect( "protocol error" )
		{
			WsEvent::Error( e ) => assert_eq!( &ErrorKind::Protocol, e.kind() ),
			evt                 => assert!( false, "{:?}", evt ),
		}
	};


	let client = async
	{
		let url         = Url::parse( "ws://127.0.0.1:3016" ).unwrap();
		let (socket, _) = ok( url ).and_then( connect_async ).compat().await.expect( "ws handshake" );

		let mut socket = socket.sink_compat();

		socket.send( tungstenite::Message::Ping( vec![1;126] ) ).await.expect( "send ping" );

		socket.close().await.expect( "close client end" );

		let frame = CloseFrame
		{
			code  : CloseCode::Protocol            ,
			reason: "Control frame too big".into() ,
		};

		assert_eq!( Some( tungstenite::Message::Close( Some(frame) )), socket.next().await.transpose().expect( "close" ) );

		trace!( "drop websocket" );
	};

	block_on( join( server, client ) );
}

