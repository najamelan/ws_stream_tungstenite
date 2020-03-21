// Verify the correct error is returned when sending a text message.
//
use
{
	ws_stream_tungstenite :: { *                                                                                  } ,
	std                   :: { future::Future                                                                     } ,
	futures               :: { StreamExt, SinkExt, executor::block_on, future::join                               } ,
	futures_codec         :: { LinesCodec, Framed                                                                 } ,
	async_tungstenite     :: { WebSocketStream                                                                    } ,
	tungstenite           :: { protocol::{ WebSocketConfig, CloseFrame, frame::coding::CloseCode, Role }, Message } ,
	pharos                :: { Observable, ObserveConfig                                                          } ,
	assert_matches        :: { assert_matches                                                                     } ,
	async_progress        :: { Progress                                                                           } ,
	futures_ringbuf       :: { Endpoint                                                                           } ,
	log                   :: { *                                                                                  } ,
};


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
	// flexi_logger::Logger::with_str( "send_text_backpressure=trace, async_progress=trace, tungstenite=warn, tokio_tungstenite=warn, ws_stream_tungstenite=warn, tokio=warn" ).start().expect( "flexi_logger");

	let (sc, cs) = Endpoint::pair( 37, 22 );

	let steps       = Progress::new( Step::FillQueue );
	let send_text   = steps.once( Step::SendText   );
	let read_text   = steps.once( Step::ReadText   );
	let client_read = steps.once( Step::ClientRead );


	let server = server( read_text, steps.clone(), sc );
	let client = client( send_text, client_read, steps.clone(), cs );

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
	let conf = WebSocketConfig
	{
		max_send_queue  : Some( 1 ),
		max_message_size: None     ,
		max_frame_size  : None     ,
	};

	let     tws    = WebSocketStream::from_raw_socket( sc, Role::Server, Some(conf) ).await;
	let mut ws     = WsStream::new( tws );
	let mut events = ws.observe( ObserveConfig::default() ).expect( "observe server" );

	let (mut sink, mut stream) = Framed::new( ws, LinesCodec {} ).split();

	let writer = async
	{
		info!( "start sending first message" );

		sink.send( "hi this is like 35 characters long\n".to_string() ).await.expect( "Send first line" );
		info!( "finished sending first message" );

		// The next step will block, so set progress
		//
		steps.set_state( Step::SendText ).await;

		sink.send( "ho\n".to_string() ).await.expect( "Send second line" );

		trace!( "server: writer end" );
	};


	let reader = async
	{
		info!( "wait for read_text" );
		read_text.await;

		// The next step will block, so set progress
		//
		steps.set_state( Step::ClientRead ).await;

		warn!( "read the text on server, should return None" );

		let res = stream.next().await.transpose();

		assert!( res.is_ok()  );
		assert!( res.unwrap().is_none()  );


		info!( "assert protocol error" );

		match events.next().await.expect( "protocol error" )
		{
			WsEvent::Error( e ) => assert!(matches!( *e, WsErr::ReceivedText )),
			evt                 => assert!( false, "{:?}", evt ),
		}

		trace!( "server: reader end" );
	};

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
	let conf = WebSocketConfig
	{
		max_send_queue  : Some( 1 ),
		max_message_size: None     ,
		max_frame_size  : None     ,
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

	assert_eq!( Some( tungstenite::Message::Close( Some(frame) )), stream.next().await.transpose().expect( "close" ) );

	// As tungstenite needs input to send out the response to the close frame, we need to keep polling
	//
	assert_eq!( None, stream.next().await.transpose().expect( "close" ) );

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
