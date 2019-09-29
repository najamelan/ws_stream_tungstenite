mod endpoint;
use endpoint::Endpoint;

// Verify the correct error is returned when sending a text message.
//
use
{
	ws_stream_tungstenite :: { *                                                                                        } ,
	futures               :: { StreamExt, SinkExt, executor::block_on, future::{ join, join3 }, compat::Sink01CompatExt } ,
	futures_codec         :: { LinesCodec, Framed                                                                       } ,
	futures::compat       :: { Stream01CompatExt                                                                        } ,
	futures_01            :: { stream::Stream                                                                           } ,
	tokio_tungstenite     :: { WebSocketStream                                                                          } ,
	tungstenite           :: { protocol::{ WebSocketConfig, CloseFrame, frame::coding::CloseCode, Role }, Message       } ,
	pharos                :: { Observable, ObserveConfig, Events                                                        } ,
	assert_matches        :: { assert_matches                                                                           } ,
	async_progress        :: { Progress                                                                                 } ,

	log           :: { * } ,
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
	// flexi_logger::Logger::with_str( "send_text_backpressure=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let (sc, cs) = Endpoint::pair( 37, 22 );

	let steps       = Progress::new( Step::Start      );
	let fill_queue  = steps.wait( Step::FillQueue  );
	let send_text   = steps.wait( Step::SendText   );
	let read_text   = steps.wait( Step::ReadText   );
	let client_read = steps.wait( Step::ClientRead );


	let server = server( fill_queue, read_text  , steps.clone(), sc );
	let client = client( send_text , client_read, steps.clone(), cs );


	info!( "start test" );
	let start = steps.set_state( Step::FillQueue );

	block_on( join3( server, client, start ) );
	info!( "end test" );
}


async fn server
(
	mut fill_queue: Events<Step> ,
	mut read_text : Events<Step> ,
	    steps     : Progress<Step>  ,
	    sc        : Endpoint         ,
)
{
	let conf = WebSocketConfig
	{
		max_send_queue  : Some( 1 ),
		max_message_size: None     ,
		max_frame_size  : None     ,
	};

	let     tws    = WebSocketStream::from_raw_socket( sc, Role::Server, Some(conf) );
	let mut ws     = WsStream::new( tws );
	let mut events = ws.observe( ObserveConfig::default() ).expect( "observe server" );

	let (mut sink, mut stream) = Framed::new( ws, LinesCodec {} ).split();

	let writer = async
	{
		info!( "wait for fill_queue" );
		fill_queue.next().await;
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
		read_text.next().await;

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
			WsEvent::Error( e ) => assert_eq!( &ErrorKind::ReceivedText, e.kind() ),
			evt                 => assert!( false, "{:?}", evt ),
		}

		trace!( "server: reader end" );
	};

	join( reader, writer ).await;

	trace!( "server: drop websocket" );
}



async fn client
(
	mut send_text   : Events<Step>,
	mut client_read : Events<Step>,
	    steps       : Progress<Step> ,
	    cs          : Endpoint        ,
)
{
	let conf = WebSocketConfig
	{
		max_send_queue  : Some( 1 ),
		max_message_size: None     ,
		max_frame_size  : None     ,
	};

	let (sink, stream) = WebSocketStream::from_raw_socket( cs, Role::Client, Some(conf) ).split();

	let mut sink   = sink.sink_compat();
	let mut stream = stream.compat();

	info!( "wait for send_text" );
	send_text.next().await;
	sink.send( tungstenite::Message::Text( "Text from client".to_string() ) ).await.expect( "send text" );

	steps.set_state( Step::ReadText ).await;

	info!( "wait for client_read" );
	client_read.next().await;

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
	Start,
	FillQueue,
	SendText,
	ReadText,
	ClientRead,
}
