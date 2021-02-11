// Verify the correct error is returned when sending a text message.
//
use
{
	ws_stream_tungstenite :: { *                                                      } ,
	futures               :: { StreamExt, SinkExt, future::join                       } ,
	futures_codec         :: { LinesCodec, Framed                                     } ,
	tokio                 :: { net::{ TcpListener }                                   } ,
	async_tungstenite     :: { accept_async, tokio::{ connect_async, TokioAdapter }   } ,
	url                   :: { Url                                                    } ,
	pharos                :: { Observable, ObserveConfig                              } ,
	tungstenite           :: { protocol::{ CloseFrame, frame::coding::CloseCode }     } ,

	log :: { * } ,
};


#[ tokio::test ]
//
async fn send_text()
{
	// flexi_logger::Logger::with_str( "ping_pong=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let server = async
	{
		let socket = TcpListener::bind( "127.0.0.1:3017" ).await.expect( "bind to port" );

		let (tcp_stream, _peer_addr) = socket.accept().await.expect( "tcp connect" );
		let s          = accept_async(TokioAdapter::new(tcp_stream)).await.expect("Error during the websocket handshake occurred");
		let mut server = WsStream::new( s );
		let mut events = server.observe( ObserveConfig::default() ).expect( "observe server" );

		let mut framed = Framed::new( server, LinesCodec {} );

		let res = framed.next().await;

		assert!( res.is_none() );

		match events.next().await.expect( "protocol error" )
		{
			WsEvent::Error( e ) => assert!(matches!( *e, WsErr::ReceivedText )),
			evt                 => unreachable!( "{:?}", evt ),
		}

		assert_eq!( None, framed.next().await.transpose().expect( "receive close stream" ) );
	};


	let client = async
	{
		let url             = Url::parse( "ws://127.0.0.1:3017" ).unwrap();
		let (mut socket, _) = connect_async( url ).await.expect( "ws handshake" );

		socket.send( tungstenite::Message::Text( "Hi".to_string() ) ).await.expect( "send text" );

		socket.close( None ).await.expect( "close client end" );

		let frame = CloseFrame
		{
			code  : CloseCode::Unsupported,
			reason: "Text messages are not supported.".into(),
		};

		assert_eq!( Some( tungstenite::Message::Close( Some(frame) )), socket.next().await.transpose().expect( "close" ) );

		trace!( "drop websocket" );
	};

	join( server, client ).await;
}

