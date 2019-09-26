// Verify the correct error is returned when sending a text message.
//
use
{
	ws_stream_tungstenite :: { *                                                                                        } ,
	futures               :: { StreamExt, SinkExt, executor::block_on, future::{ join, join3 }, compat::Sink01CompatExt } ,
	futures_codec         :: { LinesCodec, Framed                                                                       } ,
	tokio                 :: { io::{ AsyncRead as AsyncRead01, AsyncWrite as AsyncWrite01 }                             } ,
	futures::compat       :: { Stream01CompatExt                                                                        } ,
	futures_01            :: { Async, stream::Stream, task::{ self, Task }                                              } ,
	tokio_tungstenite     :: { WebSocketStream                                                                          } ,
	tungstenite           :: { protocol::{ WebSocketConfig, CloseFrame, frame::coding::CloseCode, Role }, Message       } ,
	std                   :: { io, sync::Arc, fmt                                                                       } ,
	ringbuf               :: { RingBuffer, Producer, Consumer                                                           } ,
	pharos                :: { Filter, Pharos, Observable, ObserveConfig, Events                                        } ,
	future_parking_lot    :: { mutex::{ Mutex, FutureLockable }                                                         } ,
	assert_matches        :: { assert_matches                                                                                    } ,

	log           :: { * } ,
};


// Make sure the close handshake gets completed correctly if the read end detects a protocol error and
// tries to initiate a close handshake while the send queue from tungstenite is full. Eg, does the handshake
// continue when that send queue opens up.
//
// Steps:
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

	let steps       = Steps::new( Progress::Start      );
	let fill_queue  = steps.wait( Progress::FillQueue  );
	let send_text   = steps.wait( Progress::SendText   );
	let read_text   = steps.wait( Progress::ReadText   );
	let client_read = steps.wait( Progress::ClientRead );


	let server = server( fill_queue, read_text  , steps.clone(), sc );
	let client = client( send_text , client_read, steps.clone(), cs );


	info!( "start test" );
	let start = steps.set_state( Progress::FillQueue );

	block_on( join3( server, client, start ) );
	info!( "end test" );

}


async fn server
(
	mut fill_queue: Events<Progress> ,
	mut read_text : Events<Progress> ,
	    steps     : Steps<Progress>  ,
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
		steps.set_state( Progress::SendText ).await;

		sink.send( "ho\n".to_string() ).await.expect( "Send second line" );

		trace!( "server: writer end" );
	};


	let reader = async
	{
		info!( "wait for read_text" );
		read_text.next().await;

		// The next step will block, so set progress
		//
		steps.set_state( Progress::ClientRead ).await;

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
	mut send_text   : Events<Progress>,
	mut client_read : Events<Progress>,
	    steps       : Steps<Progress> ,
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

	steps.set_state( Progress::ReadText ).await;

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



pub struct Endpoint
{
	name        : &'static str               ,

	writer      : Producer<u8>               ,
	reader      : Consumer<u8>               ,

	own_rtask   : Arc<Mutex< Option<Task> >> ,
	other_rtask : Arc<Mutex< Option<Task> >> ,
	own_wtask   : Arc<Mutex< Option<Task> >> ,
	other_wtask : Arc<Mutex< Option<Task> >> ,

	own_open    : Arc<Mutex< bool >> ,
	other_open  : Arc<Mutex< bool >> ,

}


impl Endpoint
{
	/// Create a pair of endpoints, specifying the buffer size for each one. The buffer size corresponds
	/// to the buffer the respective endpoint writes to. The other will read from this one.
	//
	pub fn pair( a_buf: usize, b_buf: usize ) -> (Endpoint, Endpoint)
	{
		let ab_buf = RingBuffer::<u8>::new( a_buf );
		let ba_buf = RingBuffer::<u8>::new( b_buf );

		let (ab_writer, ab_reader) = ab_buf.split();
		let (ba_writer, ba_reader) = ba_buf.split();

		let a_rtask = Arc::new(Mutex::new( None ));
		let a_wtask = Arc::new(Mutex::new( None ));

		let b_rtask = Arc::new(Mutex::new( None ));
		let b_wtask = Arc::new(Mutex::new( None ));

		let a_open  = Arc::new(Mutex::new( true ));
		let b_open  = Arc::new(Mutex::new( true ));

		(
			Endpoint
			{
				name        : "A_Server"      ,
				writer      : ab_writer       ,
				reader      : ba_reader       ,
				own_rtask   : a_rtask.clone() ,
				other_rtask : b_rtask.clone() ,
				own_wtask   : a_wtask.clone() ,
				other_wtask : b_wtask.clone() ,

				own_open    : a_open.clone()  ,
				other_open  : b_open.clone()  ,
			},

			Endpoint
			{
				name        : "B_Client" ,
				writer      : ba_writer  ,
				reader      : ab_reader  ,
				own_rtask   : b_rtask    ,
				other_rtask : a_rtask    ,
				own_wtask   : b_wtask    ,
				other_wtask : a_wtask    ,

				own_open    : b_open.clone()  ,
				other_open  : a_open.clone()  ,
			}
		)
	}
}



impl io::Read for Endpoint
{
	fn read( &mut self, buf: &mut [u8] ) -> io::Result<usize>
	{
		if !*self.own_open.lock()
		{
			return Ok(0);
		}

		let res = match self.reader.read( buf )
		{
			Ok(n)  =>
			{
				trace!( "{} - read {} bytes", self.name, n );

				Ok( n )
			}

			Err(e) =>
			{
				match e.kind()
				{
					io::ErrorKind::WouldBlock =>
					{
						if !*self.other_open.lock()
						{
							return Ok(0);
						}

						trace!( "{} - read: wouldblock", self.name );

						let mut own_rtask = self.own_rtask.lock();

						if own_rtask.is_some()
						{
							trace!( "{} - read: overwriting reader task", self.name );
						}

						*own_rtask = Some( task::current() );
						Err(e)
					}

					_ => Err( e )
				}

			}
		};

		if let Some( t ) = self.other_wtask.lock().take()
		{
			trace!( "{} - read: waking up writer", self.name );
			t.notify();
		}

		else
		{
			trace!( "{} - read: no writer to wake up", self.name );
		}

		res
	}
}



impl io::Write for Endpoint
{
	fn write( &mut self, buf: &[u8] ) -> io::Result<usize>
	{
		if !*self.other_open.lock() || !*self.own_open.lock()
		{
			return Ok(0);
		}

		let res = match self.writer.write( buf )
		{
			Ok(n)  =>
			{
				trace!( "{} - wrote {} bytes", self.name, n );

				Ok( n )
			}

			Err(e) =>
			{
				match e.kind()
				{
					io::ErrorKind::WouldBlock =>
					{
						trace!( "{} - write: wouldblock", self.name );

						let mut own_wtask = self.own_wtask.lock();

						if own_wtask.is_some()
						{
							trace!( "{} - write: overwriting writer task", self.name );
						}

						*own_wtask = Some( task::current() );
						Err(e)
					}

					_ => Err( e ),
				}
			}
		};

		if let Some( t ) = self.other_rtask.lock().take()
		{
			trace!( "{} - write: waking up reader", self.name );
			t.notify();
		}

		else
		{
			trace!( "{} - write: no reader to wake up", self.name );
		}

		res
	}


	fn flush( &mut self ) -> io::Result<()>
	{
		let res = match self.writer.flush()
		{
			Ok(_)  =>
			{
				trace!( "{} - writer flush Ok", self.name );

				Ok(())
			}

			Err(e) =>
			{
				match e.kind()
				{
					io::ErrorKind::WouldBlock =>
					{
						trace!( "{} - flush: wouldblock", self.name );

						let mut own_wtask = self.own_wtask.lock();

						if own_wtask.is_some()
						{
							trace!( "{} - flush: overwriting writer task", self.name );
						}

						*own_wtask = Some( task::current() );
						Err(e)
					}

					_ => Err( e ),
				}
			}
		};

		if let Some( t ) = self.other_rtask.lock().take()
		{
			trace!( "{} - flush: waking up reader", self.name );
			t.notify();
		}

		else
		{
			trace!( "{} - flush: no reader to wake up", self.name );
		}

		res
	}
}


impl Drop for Endpoint
{
	fn drop( &mut self )
	{
		warn!( "{} - drop endpoint", self.name );

		*self.own_open.lock() = false;

		// The other task might still have it's consumer, so the ringbuffer
		// will wtill be around. Therefor, make sure tasks wake up, so the notice we are closed.
		//
		if let Some( t ) = self.other_rtask.lock().take()
		{
			warn!( "{} - flush: waking up reader", self.name );
			t.notify();
		}

		if let Some( t ) = self.other_wtask.lock().take()
		{
			warn!( "{} - flush: waking up reader", self.name );
			t.notify();
		}
	}
}


impl AsyncRead01 for Endpoint {}


impl AsyncWrite01 for Endpoint
{
	fn shutdown( &mut self ) -> io::Result< Async<()> >
	{
		*self.own_open.lock() = false;

		if let Some( t ) = self.other_rtask.lock().take()
		{
			warn!( "{} - shutdown, waking up reader", self.name );
			t.notify();
		}

		Ok( ().into() )
	}
}


// Steps:
//
// server fills it's send queue
// client sends a text message (which is protocol error)
// server reads text message, initiates close handshake, but queue is full
// client starts reading, should get the messages hanging on server, followed by the close handshake.
//
#[ derive( Debug, Clone, PartialEq, Eq )]
//
enum Progress
{
	Start,
	FillQueue,
	SendText,
	ReadText,
	ClientRead,
}


#[ derive( Clone ) ]
//
pub struct Steps<State> where State: 'static + Clone + Send + Sync + Eq + fmt::Debug
{
	state : Arc<Mutex<State        >>,
	pharos: Arc<Mutex<Pharos<State>>>,
}

impl<State> Steps<State> where State: 'static + Clone + Send + Sync + Eq + fmt::Debug
{
	pub fn new( state: State ) -> Self
	{
		Self
		{
			state : Arc::new( Mutex::new( state             )) ,
			pharos: Arc::new( Mutex::new( Pharos::default() )) ,
		}
	}

	pub async fn set_state( &self, new_state: State )
	{
		let mut pharos = self.pharos.future_lock().await;
		let mut state  = self.state .future_lock().await;

		warn!( "Set state to: {:?}", new_state );

		pharos.send( new_state.clone() ).await.expect( "notify" );
		*state = new_state;
	}


	fn wait( &self, state: State ) -> Events<State>
	{
		self.pharos.lock().observe( Filter::Closure( Box::new( move |s| s == &state ) ).into() ).expect( "observe" )
	}
}


impl<State> Observable<State> for Steps<State> where State: 'static + Clone + Send + Sync + Eq + fmt::Debug
{
	type Error = pharos::Error;

	fn observe( &mut self, options: ObserveConfig<State> ) -> Result< Events<State>, Self::Error >
	{
		self.pharos.lock().observe( options )
	}
}
