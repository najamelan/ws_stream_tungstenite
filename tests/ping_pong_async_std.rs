#![type_length_limit="1617743"] // https://github.com/rust-lang/rust/issues/64496

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
	ws_stream_tungstenite :: { *                                      } ,
	futures               :: { StreamExt, SinkExt, future::join       } ,
	futures_codec         :: { LinesCodec, Framed                     } ,
	async_std             :: { net::{ TcpListener }                   } ,
	async_tungstenite     :: { accept_async, async_std::connect_async } ,
	url                   :: { Url                                    } ,

	log :: { * } ,
};


#[ async_std::test ]
//
async fn ping_pong_async_std()
{
	// flexi_logger::Logger::with_str( "ping_pong=trace, tungstenite=trace, async_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let server = async
	{
		let     socket      = TcpListener::bind( "127.0.0.1:3015" ).await.expect( "bind to port" );
		let mut connections = socket.incoming();

		let tcp_stream = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = accept_async( tcp_stream ).await.expect("Error during the websocket handshake occurred");
		let server     = WsStream::new( s );

		let mut framed = Framed::new( server, LinesCodec {} );

		assert_eq!( None, framed.next().await.transpose().expect( "receive on framed" ) );
	};


	let client = async
	{
		let url             = Url::parse( "ws://127.0.0.1:3015" ).unwrap();
		let (mut socket, _) = connect_async( url ).await.expect( "ws handshake" );

		socket.send( tungstenite::Message::Ping( vec![1, 2, 3] ) ).await.expect( "send ping" );

		socket.close( None ).await.expect( "close client end" );

		assert_eq!( Some( tungstenite::Message::Pong( vec![1, 2, 3] ) ), socket.next().await.transpose().expect( "pong"  ) );
		assert_eq!( Some( tungstenite::Message::Close(None)           ), socket.next().await.transpose().expect( "close" ) );

		trace!( "drop websocket" );
	};

	join( server, client ).await;
}

