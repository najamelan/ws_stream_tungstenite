#![ cfg( feature = "tokio_io" ) ]
//
// Test using the AsyncRead/AsyncWrite from tokio
//
// ✔ frame with futures-codec
//
use
{
	ws_stream_tungstenite :: { *                                                    } ,
	futures               :: { StreamExt, SinkExt, future::join                     } ,
	tokio_util::codec     :: { LinesCodec, Framed                                   } ,
	tokio                 :: { net::{ TcpListener }                                 } ,
	async_tungstenite     :: { accept_async, tokio::{ connect_async, TokioAdapter } } ,
	url                   :: { Url                                                  } ,

	log :: { * } ,
};


#[ tokio::test ]
//
async fn tokio_codec()
{
	// flexi_logger::Logger::with_str( "futures_codec=trace, ws_stream=trace, tokio=warn" ).start().expect( "flexi_logger");

	let server = async
	{
		let mut socket: TcpListener = TcpListener::bind( "127.0.0.1:3012" ).await.expect( "bind to port" );
		let mut connections = socket.incoming();

		let tcp_stream = connections.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = accept_async( TokioAdapter(tcp_stream) ).await.expect("Error during the websocket handshake occurred");
		let server     = WsStream::new( s );

		let (mut sink, mut stream) = Framed::new( server, LinesCodec::new() ).split();

		sink.send( "A line"       .to_string() ).await.expect( "Send a line" );
		sink.send( "A second line".to_string() ).await.expect( "Send a line" );

		sink.close().await.expect( "close server" );

		let read = stream.next().await.transpose().expect( "close connection" );

		assert!( read.is_none() );
		debug!( "Server task ended" );
	};


	let client = async
	{
		let url    = Url::parse( "ws://127.0.0.1:3012" ).unwrap();
		let socket = connect_async( url ).await.expect( "ws handshake" );

		let     client = WsStream::new( socket.0 );
		let mut framed = Framed::new( client, LinesCodec::new() );


		let res = framed.next().await.expect( "Receive some" ).expect( "Receive a line" );
		assert_eq!( "A line".to_string(), res );


		let res = framed.next().await.expect( "Receive some" ).expect( "Receive a second line" );
		assert_eq!( "A second line".to_string(), res );

		let res = framed.next().await;

		assert!( res.is_none() );
		debug!( "Client task ended" );
	};

	join( server, client ).await;
}

