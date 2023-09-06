// Verify the correct error is returned when sending a text message.
//
use
{
	ws_stream_tungstenite :: { *                                                                                  } ,
	std                   :: { future::Future                                                                     } ,
	futures               :: { StreamExt, SinkExt, executor::block_on, future::join                               } ,
	asynchronous_codec    :: { LinesCodec, Framed                                                                 } ,
	async_tungstenite     :: { WebSocketStream                                                                    } ,
	tungstenite           :: { protocol::{ WebSocketConfig, CloseFrame, frame::coding::CloseCode, Role }, Message } ,
	pharos                :: { Observable, ObserveConfig                                                          } ,
	assert_matches        :: { assert_matches                                                                     } ,
	async_progress        :: { Progress                                                                           } ,
	futures_ringbuf       :: { Endpoint                                                                           } ,
	tracing               :: { *                                                                                  } ,
};

pub fn setup_tracing()
{
	tracing_log::LogTracer::init().expect( "setup tracing_log" );

	let _ = tracing_subscriber::fmt::Subscriber::builder()

		.with_env_filter( "trace" )
		// .with_timer( tracing_subscriber::fmt::time::ChronoLocal::rfc3339() )
		.json()
	   .try_init()
	;
}


// Make sure the close handshake gets completed correctly if the read end detects a protocol error and
// tries to initiate a close handshake while the send queue from tungstenite is full. Eg, does the handshake
// continue when that send queue opens up.
//
// Progress:
//
// server fills it's send queue
// client sends a text message (which is protocol error)
// server reads text message, initiates close handshake, but queue is full
// client starts reading, should get the messages hanging on server, followed by the close handshake.
//
#[ test ]
//
fn send_text_backpressure()
{
	// setup_tracing();

	let (sc, cs) = Endpoint::pair( 37, 50 );

	let steps       = Progress::new( Step::FillQueue );
	let send_text   = steps.once( Step::SendText   );
	let read_text   = steps.once( Step::ReadText   );
	let client_read = steps.once( Step::ClientRead );

	let server = server( read_text, steps.clone(), sc );
	let client = client( send_text, client_read, steps, cs ).instrument( tracing::info_span!( "client_span" ) );

	block_on( join( server, client ) );
	info!( "end test" );
}


async fn server
(
	read_text : impl Future    ,
	steps     : Progress<Step> ,
	sc        : Endpoint       ,
)
{
	let conf = WebSocketConfig{
		write_buffer_size: 0,
		..Default::default()
	};

	let     tws    = WebSocketStream::from_raw_socket( sc, Role::Server, Some(conf) ).await;
	let mut ws     = WsStream::new( tws );
	let mut events = ws.observe( ObserveConfig::default() ).await.expect( "observe server" );

	let (mut sink, mut stream) = Framed::new( ws, LinesCodec {} ).split();

	let writer = async
	{
		info!( "start sending first message" );

		sink.send( "hi this is like 35 characters long\n".to_string() ).await.expect( "Send first line" );
		info!( "finished sending first message" );

		// The next step will block, so set progress
		//
		steps.set_state( Step::SendText ).await;

		// With the message from above, this is bigger than 37 bytes, so it will block on the underlying transport.
		sink.send( "ho block\n".to_string() ).await.expect( "Send second line" );

		trace!( "server: writer end" );
	};


	let reader = async
	{
		info!( "wait for read_text" );
		read_text.await;

		// The next step will block, so set progress
		//
		steps.set_state( Step::ClientRead ).await;

		warn!( "server: read the text, should return None" );

		let res = stream.next().await;

		assert!(res.is_none());

		info!( "server: assert protocol error" );

		match events.next().await.expect( "protocol error" )
		{
			WsEvent::Error( e ) => assert!(matches!( *e, WsErr::ReceivedText )),
			evt                 => unreachable!( "{:?}", evt ),
		}

		trace!( "server: reader end" );
	};

	let reader = reader.instrument( tracing::info_span!( "server_reader" ) );
	let writer = writer.instrument( tracing::info_span!( "server_writer" ) );

	join( reader, writer ).await;

	trace!( "server: drop websocket" );
}



async fn client
(
	send_text   : impl Future    ,
	client_read : impl Future    ,
	steps       : Progress<Step> ,
	cs          : Endpoint       ,
)
{
	let conf = WebSocketConfig{
		write_buffer_size: 0,
		..Default::default()
	};

	let (mut sink, mut stream) = WebSocketStream::from_raw_socket( cs, Role::Client, Some(conf) ).await.split();

	info!( "wait for send_text" );
	send_text.await;
	sink.send( tungstenite::Message::Text( "Text from client".to_string() ) ).await.expect( "send text" );

	steps.set_state( Step::ReadText ).await;

	info!( "wait for client_read" );
	client_read.await;

	let test = stream.next().await.unwrap().unwrap();
	assert_matches!( test, Message::Binary(_) );

	info!( "client: received first binary message" );


	let test = stream.next().await.unwrap().unwrap();
	assert_matches!( test, Message::Binary(_) );

	info!( "client: received second binary message" );


	let frame = CloseFrame
	{
		code  : CloseCode::Unsupported,
		reason: "Text messages are not supported.".into(),
	};

	warn!( "client: waiting on close frame" );

	assert_eq!( Some( tungstenite::Message::Close( Some(frame) )), stream.next().await.transpose().expect( "client: receive close frame" ) );

	// As tungstenite needs input to send out the response to the close frame, we need to keep polling
	//
	assert_eq!( None, stream.next().await.transpose().expect( "client: stream closed" ) );

	trace!( "client: drop websocket" );
}



// Progress:
//
// server fills it's send queue
// client sends a text message (which is protocol error)
// server reads text message, initiates close handshake, but queue is full
// client starts reading, should get the messages hanging on server, followed by the close handshake.
//
#[ derive( Debug, Clone, PartialEq, Eq )]
//
enum Step
{
	FillQueue  ,
	SendText   ,
	ReadText   ,
	ClientRead ,
}
