mod notifier;
mod closer  ;

use
{
	crate    :: { import::*, WsEvent, ErrorKind, Error } ,
	notifier ::Notifier                                  ,
	closer   ::Closer                                    ,
};



#[ derive( Debug, Clone, PartialEq, Eq ) ]
//
enum State
{
	// All systems green.
	//
	Ready,

	// Tungstenite indicated us that the connection is closed, we should neither poll,
	// nor try to send anything to it anymore.
	//
	ConnectionClosed,
}



bitflags!
{
	/// Tasks that are woken up always come from either the poll_read (Stream) method or poll_ready and
	/// poll_flush, poll_close methods (Sink).
	///
	/// However we inject extra work (Sending a close frame from read and Sending events on pharos).
	/// When these tasks return pending, we want to return pending from our user facing poll methods.
	/// However when progress can be made these will wake up the task and it's our user facing methods
	/// that get called first. This state allows tracking which of are in progress and need to be resumed.
	//
	struct Flag: u8
	{
		const NOTIFIER_PEND = 0x01;
		const CLOSER_PEND   = 0x02;
		const PHAROS_CLOSED = 0x04;
		const SINK_CLOSED   = 0x08;
	}
}




/// A wrapper around a WebSocket provided by tungstenite. This provides Stream/Sink Vec<u8> to
/// simplify implementing AsyncRead/AsyncWrite on top of tokio-tungstenite.
///
/// The methods `close_because` and `verify_close_in_progress` are helper methods to send out
/// a close frame over the sink from poll_next, whilst first polling this until it returns Poll::Ready
/// before polling tungstenite for a next message.
///
/// Similarly, `notify` and `verify_pharos` do the same for sending out events over pharos.
///
/// These methods don't return errors (would be send specific errors) since we are mostly in reading context.
/// The close methods return errors through the event stream, and when writing to the event stream fails,
/// they silently stop sending out events.
//
pub(crate) struct TungWebSocket<S: AsyncRead01 + AsyncWrite01>
{
	sink  : Compat01As03Sink< SplitSink01  < TTungSocket<S> >, TungMessage > ,
	stream: Compat01As03    < SplitStream01< TTungSocket<S> >              > ,

	state     : State    ,
	flag      : Flag     ,
	notifier  : Notifier ,
	closer    : Closer   ,
}


impl<S> TungWebSocket<S> where S: AsyncRead01 + AsyncWrite01
{
	/// Create a new Wrapper for a WebSocket provided by Tungstenite
	//
	pub(crate) fn new( inner: TTungSocket<S> ) -> Self
	{
		let (tx, rx) = inner.split();

		Self
		{
			stream   : Compat01As03    ::new( rx ) ,
			sink     : Compat01As03Sink::new( tx ) ,

			state    : State::Ready                ,
			flag     : Flag::empty()               ,
			notifier : Notifier::new()             ,
			closer   : Closer::new()               ,
		}
	}


	// Check whether there is messages queued up for notification.
	// Return Pending until all of them are processed.
	//
	fn check_notify( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<()>
	{
		if !self.flag.contains( Flag::NOTIFIER_PEND )
		{
			return ().into();
		}

		match ready!( self.notifier.run( cx ) )
		{
			Ok (_) => {}
			Err(_) => self.flag.insert( Flag::PHAROS_CLOSED ),
		}

		self.flag.remove( Flag::NOTIFIER_PEND );

		().into()
	}


	fn queue_event( &mut self, evt: WsEvent )
	{
		// It should only happen if we call close on it, and we should never do that.
		//
		debug_assert!( !self.flag.contains( Flag::PHAROS_CLOSED ), "If this happens, it's a bug in ws_stream_tungstenite, please report." );

		self.notifier.queue( evt );

		self.flag.insert( Flag::NOTIFIER_PEND );
	}


	// Take care of sending a close frame to tungstenite.
	//
	// Will return pending until the entire operation is finished.
	//
	fn send_closeframe( &mut self, code: CloseCode, reason: Cow<'static, str>, cx: &mut Context<'_> ) -> Poll<()>
	{
		self.flag.insert( Flag::CLOSER_PEND );

		// As soon as we are closing, accept no more messages for writing.
		//
		self.flag.insert( Flag::SINK_CLOSED );

		// TODO: check for connection closed, sink errors, etc

		self.closer.queue( CloseFrame{ code, reason } );

		if ready!( Pin::new( &mut self.closer ).run( &mut self.sink, &mut self.notifier, cx) ).is_err()
		{
			self.flag.insert( Flag::SINK_CLOSED );
		}

		self.flag.remove( Flag::CLOSER_PEND );

		Pin::new( self ).check_notify( cx )
	}


	// Check whether there is messages queued up for notification.
	// Return Pending until all of them are processed.
	//
	fn check_closer( &mut self, cx: &mut Context<'_> ) -> Poll<()>
	{
		if !self.flag.contains( Flag::CLOSER_PEND )
		{
			return ().into();
		}


		if ready!( Pin::new( &mut self.closer ).run( &mut self.sink, &mut self.notifier, cx) ).is_err()
		{
			self.flag.insert( Flag::SINK_CLOSED );
		}


		self.flag.remove( Flag::CLOSER_PEND );

		().into()
	}
}



impl<S: AsyncRead01 + AsyncWrite01> Stream for TungWebSocket<S>
{
	type Item = Result<Vec<u8>, io::Error>;


	/// Get the next websocket message and convert it to a Vec<u8>.
	///
	/// When None is returned, it means it is safe to drop the underlying connection.
	///
	/// ### Errors
	///
	/// The following errors can be returned from this method:
	///
	/// - [`io::ErrorKind::InvalidData`]: This means that a protocol error was encountered (eg. the remote did something wrong)
	///   Protocol error here means either against the websocket protocol or sending websocket Text messages which don't
	///   make sense for AsyncRead/AsyncWrite. When these happen, we start the close handshake for the connection, notifying
	///   the remote that they made a protocol violation. You should not drop the connection and keep polling this stream
	///   until it returns None. That allows tungstenite to properly handle shutting down the connection.
	///   Until the remote confirms the close, we might still receive incoming data and it will be passed on to you.
	///   In production code you probably want to stop polling this after a timeout if the remote doesn't acknowledge the
	///   close.
	///
	/// - other std::io::Error's generally mean something went wrong on the underlying transport. Consider these fatal
	///   and just drop the connection.
	//
	fn poll_next( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Option<Self::Item> >
	{
		trace!( "TungWebSocket::poll_next" );

		// Events can provide back pressure with bounded channels. If this is pending, we don't
		// do anything else that might generate more events before these have been delivered.
		//
		ready!( self.as_mut().check_notify( cx ) );

		// If we are in the middle of sending out a close frame, make sure that is finished before
		// doing any further polling for incoming messages.
		//
		// After check_closer finishes it's own work, it calls check_notify because it might have
		// added new events to the queue.
		//
		ready!( self.as_mut().check_closer( cx ) );

		// Never poll tungstenite if we already got note that the connection is ready to be dropped.
		//
		if self.state == State::ConnectionClosed
		{
			return None.into();
		}


		trace!( "poll tokio-tungstenite for read" );

		// Do actual reading from stream.
		//
		let res = ready!( Pin::new( &mut self.stream ).poll_next( cx ) );

		trace!( "matching message type: {:?}", res );

		match res
		{
			None =>
			{
				trace!( "poll_next: got None from inner stream" );

				// if tungstenite is returning None here, we should no longer try to send a pending close frame.
				//
				self.flag.remove( Flag::CLOSER_PEND );

				self.state = State::ConnectionClosed;
				None.into()
			}


			Some(Ok( msg )) =>
			{
				match msg
				{
					TungMessage::Binary(vec) => Some(Ok( vec )).into(),


					TungMessage::Text(_) =>
					{
						warn!( "Received text message, will close the connection." );

						self.queue_event( WsEvent::Error(Arc::new( Error::from(ErrorKind::ReceivedText) )) );

						let string = "Text messages are not supported.";

						// If this returns pending, we don't want to recurse, the task will be woken up.
						//
						ready!( self.as_mut().send_closeframe( CloseCode::Unsupported, string.into(), cx ) );

						// Continue to drive the event and the close handshake before returning.
						//
						self.poll_next( cx )
					}


					TungMessage::Close(opt) =>
					{
						debug!( "received close frame: {:?}", opt );

						self.queue_event( WsEvent::CloseFrame( opt ));

						// Tungstenite will keep this stream around until the underlying connection closes.
						// It's important we don't return None here so clients don't drop the underlying connection
						// while the other end is still processing stuff, otherwise they receive a connection reset
						// error and can't read any more data waiting to be processed.
						//
						self.poll_next( cx )
					}


					// Tungstenite will have answered it already
					//
					TungMessage::Ping(data) =>
					{
						self.queue_event( WsEvent::Ping(data) );
						self.poll_next( cx )
					}

					TungMessage::Pong(data) =>
					{
						self.queue_event( WsEvent::Pong(data) );
						self.poll_next( cx )
					}
				}
			}



			Some(Err( err )) =>
			{
				match err
				{
					// Just return None, as no more data will come in.
					// This can mean tungstenite state is Terminated and we can safely drop the underlying connection.
					// Note that tungstenite only set's this on the client after the server has closed the underlying
					// connection, to comply with the RFC.
					//
					TungErr::ConnectionClosed |
					TungErr::AlreadyClosed =>
					{
						trace!( "poll_next: got {} from inner stream", err );

						debug_assert!( self.state != State::ConnectionClosed, "We polled tungstenite after the connection was closed, this is a bug in ws_stream_tungstenite, please report." );

						self.state = State::ConnectionClosed;

						self.queue_event( WsEvent::Closed );

						self.poll_next( cx )
					}


					// This generally means the underlying transport is broken. Tungstenite will keep bubbling up the
					// same error over and over, consider this fatal.
					//
					TungErr::Io(e) =>
					{
						self.state = State::ConnectionClosed;

						Some(Err(e)).into()
					}

					// In principle this can fail. If the sendqueue of tungstenite is full, it will return
					// an error and the close frame will stay in the Send future, or in the buffer of the
					// compat sink, but the point is that it's impossible to create a full send queue with
					// the API we provide.
					//
					// On every call to write on WsStream, we create a full ws message and the poll_write
					// only
					//
					TungErr::Protocol( ref string ) =>
					{
						error!( "Protocol error from Tungstenite: {}", string );

						// If this returns pending, we don't want to recurse, the task will be woken up.
						//
						ready!( self.as_mut().send_closeframe( CloseCode::Protocol, string.clone(), cx ) );


						self.queue_event( WsEvent::Error( Arc::new( Error::from(err) )) );


						// Continue to drive the event and the close handshake before returning.
						//
						self.poll_next( cx )
					}

					// This also means the remote sent a text message which isn't supported anyway, so we don't much care
					// for the utf errors
					//
					TungErr::Utf8 =>
					{
						error!( "{}", &err );

						let string = "Text messages are not supported";

						self.queue_event( WsEvent::Error( Arc::new( Error::from(err) )) );

						// If this returns pending, we don't want to recurse, the task will be woken up.
						//
						ready!( self.as_mut().send_closeframe( CloseCode::Unsupported, string.into(), cx ) );

						// Continue to drive the event and the close handshake before returning.
						//
						self.poll_next( cx )
					}


					// TODO: actually capacity is on incoming as well
					//
					TungErr::Capacity(_) =>
					{
						error!( "{}", &err );

						self.queue_event( WsEvent::Error( Arc::new( Error::from(err) )) );
						self.poll_next( cx )
					}


					// I hope none of these can occur here because they are either handshake errors
					// or buffer capacity errors.
					//
					// This should only happen in the write side:
					//
					TungErr::SendQueueFull(_) |

					// These are handshake errors:
					//
					TungErr::Tls (_)          |
					TungErr::Url (_)          |
					TungErr::Http(_)          =>

						unreachable!( "{:?}", err ),

				}
			}
		}
	}
}



// TODO: How to communicate to the user and know when the connection will be closed. Is it possible
// to get an error on sending, but just polling the stream will not suffice to close the connection?
// eg. are there situations where the user still has to close manually? In principle a websocket is
// a duplex connection, so we probably should not let this happen.
//
impl<S: AsyncRead01 + AsyncWrite01> Sink<Vec<u8>> for TungWebSocket<S>
{
	type Error = io::Error;


	fn poll_ready( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		// If we were busy closing, first finish that. Will return on pending or OK.
		//
		ready!( self.as_mut().check_closer( cx ) );

		// Are there any events waiting for which we should inform observers?
		//
		ready!( self.as_mut().check_notify( cx ) );


		if self.flag.contains( Flag::SINK_CLOSED )
		{
			return Err( io::ErrorKind::NotConnected.into() ).into()
		}


		Pin::new( &mut self.sink ).poll_ready( cx ).map_err( to_io_error )
	}


	/// ### Errors
	///
	/// The following errors can be returned when writing to the stream:
	///
	/// - [`io::ErrorKind::NotConnected`]: This means that the connection is already closed. You should
	///   no longer write to it. It is safe to drop the underlying connection when `poll_next` returns None.
	///
	///   TODO: if error capacity get's returned, is the socket still usable?
	///
	/// - [`io::ErrorKind::InvalidData`]: This means that a tungstenite::error::Capacity occurred. This means that
	///   you send in a buffer bigger than the maximum message size configured on the underlying websocket connection.
	///   If you did not set it manually, the default for tungstenite is 64MB.
	///
	/// - other std::io::Error's generally mean something went wrong on the underlying transport. Consider these fatal
	///   and just drop the connection as soon as `poll_next` returns None.
	//
	fn start_send( mut self: Pin<&mut Self>, item: Vec<u8> ) -> Result<(), Self::Error>
	{
		trace!( "TungWebSocket: start_send" );

		if self.flag.contains( Flag::SINK_CLOSED )
		{
			return Err( io::ErrorKind::NotConnected.into() ).into()
		}

		Pin::new( &mut self.sink ).start_send( item.into() ).map_err( to_io_error )
	}

	/// This will do a send under the hood, so the same errors as from start_send can occur here.
	//
	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		trace!( "TungWebSocket: poll_flush" );

		Pin::new( &mut self.sink ).poll_flush( cx ).map_err( to_io_error )
	}


	/// Will resolve immediately. Keep polling the stream until it returns None. To make sure
	/// to keep the underlying connection alive until the close handshake is finished.
	///
	/// This will do a send under the hood, so the same errors as from start_send can occur here,
	/// except InvalidData.
	//
	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		trace!( "TungWebSocket: poll_close" );

		self.flag.insert( Flag::SINK_CLOSED );

		// We ignore closed errors since that's what we want, and because after calling this method
		// the sender task can in any case be dropped, and verifying that the connection can actually
		// be closed should be done through the reader task.
		//
		Pin::new( &mut self.sink ).poll_close( cx ).map_err( to_io_error )
	}
}



// Convert tungstenite errors that can happen during sending into io::Error.
//
fn to_io_error( err: TungErr ) -> io::Error
{
	error!( "{:?}", &err );

	match err
	{
		// Mainly on the underlying stream. Fatal
		//
		TungErr::Io(err) => err,


		// Connection is closed, does not indicate something went wrong.
		//
		TungErr::ConnectionClosed |

		// Connection is closed, in principle this indicates that the user tries to keep using it
		// after ConnectionClosed has already been returned.
		//
		TungErr::AlreadyClosed => io::ErrorKind::NotConnected.into() ,


		// This shouldn't happen, we should not cause any protocol errors, since we abstract
		// away the websocket protocol for users. They shouldn't be able to trigger this through our API.
		//
		TungErr::Protocol(_string) =>
		{
			unreachable!( "protocol error from tungstenite on send is bug in ws_stream_tungstenite, please report" );
			// io::Error::new( io::ErrorKind::ConnectionReset, string )
		}


		// This can happen when we create a message bigger than max message size in tungstenite.
		//
		TungErr::Capacity(string) => io::Error::new( io::ErrorKind::InvalidData, string ),


		// This is dealt with by backpressure in the compat layer over tokio-tungstenite.
		// We should never see this error.
		//
		TungErr::SendQueueFull(_) |

		// These are handshake errors
		//
		TungErr::Tls (..) |
		TungErr::Http(..) |
		TungErr::Url (..) |

		// This is an error specific to Text Messages that we don't use
		//
		TungErr::Utf8 => unreachable!() ,
	}
}


impl<S> Observable< WsEvent > for TungWebSocket<S> where S: AsyncRead01 + AsyncWrite01
{
	type Error = Error;

	fn observe( &mut self, options: ObserveConfig< WsEvent > ) -> Result< Events< WsEvent >, Self::Error >
	{
		self.notifier.observe( options ).map_err( Into::into )
	}
}




