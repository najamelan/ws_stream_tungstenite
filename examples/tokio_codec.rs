#![ cfg( feature = "tokio_io" ) ]
//!
//! Example showing how the stream can be framed with tokio codec.
//
use
{
	tungstenite           :: { protocol::Role                   } ,
	ws_stream_tungstenite :: { *                                } ,
	futures               :: { StreamExt, SinkExt, future::join } ,
	tokio_util::codec     :: { LinesCodec, Framed               } ,
	futures_ringbuf       :: { Endpoint                         } ,
	tracing               :: { *                                } ,
};


#[ tokio::main ]
//
async fn main()
{
	let (server_con, client_con) = Endpoint::pair( 64, 64 );

	let server = async move
	{
		let s          = async_tungstenite::WebSocketStream::from_raw_socket( server_con, Role::Server, None ).await;
		let server     = WsStream::new( s );

		let mut framed = Framed::new( server, LinesCodec::new() );

		framed.send( "A line"       .to_string() ).await.expect( "Send a line" );
		framed.send( "A second line".to_string() ).await.expect( "Send a line" );

		debug!( "closing server side" );
		<Framed<WsStream<Endpoint>, LinesCodec> as futures::SinkExt<String>>::close( &mut framed ).await.expect( "close server" );
		debug!( "closed server side" );

		let read = framed.next().await.transpose().expect( "close connection" );

		assert!( read.is_none() );
		debug!( "Server task ended" );
	};


	let client = async move
	{
		let socket = async_tungstenite::WebSocketStream::from_raw_socket( client_con, Role::Client, None ).await;

		let     client = WsStream::new( socket );
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
