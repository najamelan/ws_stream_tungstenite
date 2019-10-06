
// Tested:
//
// ✔ notifiying errors through pharos
// - dealing with backpressure and send in several times
//
use crate :: { import::*, WsEvent, ErrorKind, tung_websocket::{ notifier::Notifier, closer::Closer } };


#[ test ]
//
fn notify_errors()
{
	// flexi_logger::Logger::with_str( "send_text_backpressure=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let (sc, cs) = Endpoint::pair( 100, 100 );


	let test = async
	{
		let mut sink   = WebSocketStream::from_raw_socket( sc, Role::Server, None ).split().0.sink_compat();
		let mut stream = WebSocketStream::from_raw_socket( cs, Role::Client, None ).split().1.compat();

		let mut notif  = Notifier::new();
		let mut events = notif.observe( ObserveConfig::default() ).expect( "observe server" );
		let mut closer = Closer::new();
		let     waker  = noop_waker();
		let mut cx     = Context::from_waker( &waker );

		let writer = async
		{
			sink.close().await.expect( "close sink" );

			closer.queue( CloseFrame{ code: CloseCode::Unsupported, reason: "tada".into() } ).expect( "queue close" );

			// this will encounter an error since the sink is already closed
			//
			let p = Pin::new( &mut closer ).run( &mut sink, &mut notif, &mut cx );

			assert_matches!( p, Poll::Ready( Err(()) ) );

			let n = notif.run( &mut cx );

			assert_matches!( n, Poll::Ready( Ok(()) ) );

			trace!( "server: writer end" );
		};


		let reader = async
		{
			trace!( "client: waiting on close frame" );

			assert_eq!( Some( tungstenite::Message::Close( None )), stream.next().await.transpose().expect( "close" ) );

			// Verify the error can be observed here
			//
			match events.next().await.expect( "error" )
			{
				WsEvent::Error( e ) =>
				{
					assert_eq!( &ErrorKind::Protocol, e.kind() );
				}

				_ => unreachable!( "expect closeframe" ),
			}

			trace!( "client: reader end" );
		};

		join( reader, writer ).await;

		trace!( "client: drop websocket" );
	};

	block_on( test );
	info!( "end test" );
}




