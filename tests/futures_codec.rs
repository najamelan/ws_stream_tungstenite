// Test using the AsyncRead/AsyncWrite from futures 0.3
//
// ✔ frame with futures-codec
//
use
{
	ws_stream_tungstenite :: { *                                                            } ,
	futures               :: { StreamExt, SinkExt, executor::block_on, future::join         } ,
	futures_codec         :: { LinesCodec, Framed                                           } ,
	tokio                 :: { net::{ TcpListener }                                         } ,
	futures::compat       :: { Future01CompatExt, Stream01CompatExt                         } ,
	futures_01            :: { future::{ ok, Future as _ }                                  } ,
	tokio_tungstenite     :: { accept_async, connect_async                                  } ,
	url                   :: { Url                                                          } ,

	log :: { * } ,
};


#[ test ]
//
fn frame03()
{
	// flexi_logger::Logger::with_str( "futures_codec=trace, ws_stream=trace, tokio=warn" ).start().expect( "flexi_logger");

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

		assert!( read.is_none() );
		debug!( "Server task ended" );
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

		assert!( res.is_none() );
		debug!( "Client task ended" );
	};

	block_on( join( server, client ) );
}

