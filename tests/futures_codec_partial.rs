// Test using the AsyncRead/AsyncWrite from futures 0.3
//
// ✔ send/receive half a frame
//
use
{
	ws_stream_tungstenite :: { *                                                    } ,
	futures               :: { StreamExt, SinkExt, future::join, channel::oneshot   } ,
	futures_codec         :: { LinesCodec, Framed                                   } ,
	tokio                 :: { net::{ TcpListener }                                 } ,
	async_tungstenite     :: { accept_async, tokio::{ connect_async, TokioAdapter } } ,
	url                   :: { Url                                                  } ,

	log :: { * } ,
};



// Receive half a frame
//
#[ tokio::test ]
//
async fn partial()
{
	// flexi_logger::Logger::with_str( "futures_codec=trace, tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let (tx, rx) = oneshot::channel();

	let server = async move
	{
		let mut socket = TcpListener::bind( "127.0.0.1:3013" ).await.expect( "bind to port" );
		let tcp_stream = socket.next().await.expect( "1 connection" ).expect( "tcp connect" );
		let s          = accept_async(TokioAdapter(tcp_stream)).await.expect("Error during the websocket handshake occurred");
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

		assert!( read.is_none() );
		debug!( "Server task ended" );
	};


	let client = async move
	{
		let url    = Url::parse( "ws://127.0.0.1:3013" ).unwrap();
		let socket = connect_async( url ).await.expect( "ws handshake" );

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
		assert!( res.is_none() );
		debug!( "Client task ended" );
	};

	join( server, client ).await;
}
