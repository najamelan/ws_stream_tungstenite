// Verify the correct error is returned when sending a protocol error.
//
use
{
	ws_stream_tungstenite :: { *                                                    } ,
	futures               :: { StreamExt, SinkExt, future::join                     } ,
	asynchronous_codec    :: { LinesCodec, Framed                                   } ,
	tokio                 :: { net::{ TcpListener }                                 } ,
	async_tungstenite     :: { accept_async, tokio::{ connect_async, TokioAdapter } } ,
	url                   :: { Url                                                  } ,
	pharos                :: { Observable, ObserveConfig                            } ,
	tungstenite           :: { protocol::{ CloseFrame, frame::coding::CloseCode }   } ,
	tracing               :: { *                                                    } ,
};


#[ tokio::test ]
//
async fn protocol_error()
{
	// flexi_logger::Logger::with_str( "ping_pong=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");


	let server = async
	{
		let socket: TcpListener = TcpListener::bind( "127.0.0.1:3016" ).await.expect( "bind to port" );

		let (tcp_stream, _peer_addr) = socket.accept().await.expect( "tcp connect" );
		let s          = accept_async(TokioAdapter::new(tcp_stream)).await.expect("Error during the websocket handshake occurred");
		let mut server = WsStream::new( s );

		let mut events = server.observe( ObserveConfig::default() ).await.expect( "observe" );
		let mut framed = Framed::new( server, LinesCodec {} );


		assert!( framed.next().await.is_none() );

		match events.next().await.expect( "protocol error" )
		{
			WsEvent::Error( e ) => assert!(matches!( *e, WsErr::Protocol )),
			evt                 => unreachable!( "{:?}", evt ),
		}
	};


	let client = async
	{
		let url             = Url::parse( "ws://127.0.0.1:3016" ).unwrap();
		let (mut socket, _) = connect_async( url ).await.expect( "ws handshake" );

		socket.send( tungstenite::Message::Ping( vec![1;126] ) ).await.expect( "send ping" );

		socket.close( None ).await.expect( "close client end" );

		let frame = CloseFrame
		{
			code  : CloseCode::Protocol            ,
			reason: "Control frame too big (payload must be 125 bytes or less)".into() ,
		};

		assert_eq!
		(
			Some( tungstenite::Message::Close( Some(frame) )),
			socket.next().await.transpose().expect( "close" )
		);

		trace!( "drop websocket" );
	};

	join( server, client ).await;
}

