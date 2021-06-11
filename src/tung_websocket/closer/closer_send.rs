// Tested:
//
// ✔ closer actually sends out on sink
//
use crate :: { import::{ *, assert_matches }, tung_websocket::{ notifier::Notifier, closer::Closer } };


#[ test ]
//
fn send_closeframe()
{
	// flexi_logger::Logger::with_str( "send_text_backpressure=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let (sc, cs) = Endpoint::pair( 100, 100 );


	let test = async
	{
		let mut sink   = ATungSocket::from_raw_socket( sc, Role::Server, None ).await.split().0;
		let mut stream = ATungSocket::from_raw_socket( cs, Role::Client, None ).await.split().1;

		let mut notif  = Notifier::new();
		let mut closer = Closer::new();
		let     waker  = noop_waker();
		let mut cx     = Context::from_waker( &waker );

		let writer = async
		{
			closer.queue( CloseFrame{ code: CloseCode::Unsupported, reason: "tada".into() } )

				.expect( "no double close" )
			;

			let p = Pin::new( &mut closer ).run( &mut sink, &mut notif, &mut cx );

			assert_matches!( p, Poll::Ready( Ok(()) ) );

			trace!( "server: writer end" );
		};


		let reader = async
		{
			let frame = CloseFrame
			{
				code  : CloseCode::Unsupported,
				reason: "tada".into(),
			};

			trace!( "client: waiting on close frame" );

			assert_eq!( Some( tungstenite::Message::Close( Some(frame) )), stream.next().await.transpose().expect( "close" ) );

			trace!( "client: reader end" );
		};

		join( reader, writer ).await;

		trace!( "client: drop websocket" );
	};

	block_on( test );
	info!( "end test" );
}




