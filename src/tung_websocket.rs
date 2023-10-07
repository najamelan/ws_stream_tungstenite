mod notifier;
mod closer  ;

use
{
	crate    :: { import::*, WsEvent, WsErr } ,
	notifier :: { Notifier                  } ,
	closer   :: { Closer                    } ,
};


bitflags!
{
	/// Tasks that are woken up always come from either the poll_read (Stream) method or poll_ready and
	/// poll_flush, poll_close methods (Sink).
	///
	/// However we inject extra work (Sending a close frame from read and Sending events on pharos).
	/// When these tasks return pending, we want to return pending from our user facing poll methods.
	/// However when progress can be made these will wake up the task and it's our user facing methods
	/// that get called first. This state allows tracking which sub tasks are in progress and need to be resumed.
	///
	/// SINK_CLOSED is used to keep track of any state where we should no longer send anything into the sink
	/// (eg. it returned an error). In that case, we might still poll the stream to drive a close handshake
	/// to completion.
	//
	struct State: u8
	{
		const NOTIFIER_PEND = 0x01;
		const CLOSER_PEND   = 0x02;
		const PHAROS_CLOSED = 0x04;
		const SINK_CLOSED   = 0x08;
		const STREAM_CLOSED = 0x10;
	}
}




/// A wrapper around a WebSocket provided by tungstenite. This provides Stream/Sink Vec<u8> to
/// simplify implementing AsyncRead/AsyncWrite on top of async-tungstenite.
//
pub(crate) struct TungWebSocket<S>  where S: AsyncRead + AsyncWrite + Send + Unpin
{
	inner: ATungSocket<S> ,

	state    : State    ,
	notifier : Notifier ,
	closer   : Closer   ,
}


impl<S> TungWebSocket<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	/// Create a new Wrapper for a WebSocket provided by Tungstenite
	//
	pub(crate) fn new( inner: ATungSocket<S> ) -> Self
	{
		Self
		{
			inner                        ,
			state    : State   ::empty() ,
			notifier : Notifier::new()   ,
			closer   : Closer  ::new()   ,
		}
	}


	// Check whether there is messages queued up for notification.
	// Returns Pending until all of them are processed.
	//
	fn check_notify( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<()>
	{
		if !self.state.contains( State::NOTIFIER_PEND )
		{
			return ().into();
		}

		match ready!( self.notifier.run( cx ) )
		{
			Ok (_) => {}
			Err(_) => self.state.insert( State::PHAROS_CLOSED ),
		}

		self.state.remove( State::NOTIFIER_PEND );

		().into()
	}


	// Queue a new event to be delivered to observers.
	//
	fn queue_event( &mut self, evt: WsEvent )
	{
		// It should only happen if we call close on it, and we should never do that.
		//
		debug_assert!
		(
			!self.state.contains( State::PHAROS_CLOSED ),
			"If this happens, it's a bug in ws_stream_tungstenite, please report."
		);

		self.notifier.queue( evt );

		self.state.insert( State::NOTIFIER_PEND );
	}


	// Take care of sending a close frame to tungstenite.
	//
	// Will return pending until the entire sending operation is finished. We still need to poll
	// the stream to drive the handshake to completion.
	//
	fn send_closeframe( &mut self, code: CloseCode, reason: Cow<'static, str>, cx: &mut Context<'_> ) -> Poll<()>
	{
		// If the sink is already closed, don't try to send any more close frames.
		//
		if !self.state.contains( State::SINK_CLOSED )
		{
			// As soon as we are closing, accept no more messages for writing.
			//
			self.state.insert( State::SINK_CLOSED );
			self.state.insert( State::CLOSER_PEND );

			self.closer.queue( CloseFrame{ code, reason } )

				.expect( "ws_stream_tungstenite should not queue 2 close frames" )
			;
		}

		self.check_closer( cx )
	}


	// Check whether there is a close frame in progress of being sent.
	// Returns Pending the underlying sink is flushed.
	//
	fn check_closer( &mut self, cx: &mut Context<'_> ) -> Poll<()>
	{
		if !self.state.contains( State::CLOSER_PEND )
		{
			return ().into();
		}


		if ready!( Pin::new( &mut self.closer ).run( &mut self.inner, &mut self.notifier, cx) ).is_err()
		{
			self.state.insert( State::SINK_CLOSED );
		}

		self.state.remove( State::CLOSER_PEND );


		// Since closer might have queued events, before returning, make sure they are flushed.
		//
		Pin::new( self ).check_notify( cx )
	}
}



impl<S: Unpin> Stream for TungWebSocket<S> where S: AsyncRead + AsyncWrite + Send
{
	type Item = Result<Vec<u8>, io::Error>;


	/// Get the next websocket message and convert it to a Vec<u8>.
	///
	/// When None is returned, it means it is safe to drop the underlying connection. Even after calling
	/// close on the sink, this should be polled until it returns None to drive the close handshake to completion.
	///
	/// ### Errors
	///
	/// Errors are mostly returned out of band as events through pharos::Observable. Only fatal errors are returned
	/// directly from this method.
	///
	/// The following errors can be returned from this method:
	///
	/// - std::io::Error generally mean something went wrong on the underlying transport. Consider these fatal
	///   and just drop the connection.
	//
	fn poll_next( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Option<Self::Item> >
	{
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
		if self.state.contains( State::STREAM_CLOSED )
		{
			return None.into();
		}


		// Do actual reading from stream.
		//
		let res = ready!( Pin::new( &mut self.inner ).poll_next( cx ) );


		match res
		{
			None =>
			{
				// if tungstenite is returning None here, we should no longer try to send a pending close frame.
				//
				self.state.remove( State::CLOSER_PEND   );
				self.state.insert( State::STREAM_CLOSED );

				None.into()
			}


			Some(Ok( msg )) =>
			{
				match msg
				{
					TungMessage::Binary(vec) => Some(Ok( vec )).into(),


					TungMessage::Text(_) =>
					{
						self.queue_event( WsEvent::Error(Arc::new( WsErr::ReceivedText )) );

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

					TungMessage::Frame(_) =>
					{
						unreachable!( "A Message::Frame(..) should be never occur from a read" );
					}
				}
			}



			Some(Err( err )) =>
			{
				// See the wildcard at the bottom for why we need this.
				//
				#[ allow( unreachable_patterns, clippy::wildcard_in_or_patterns )]
				//
				match err
				{
					// Just return None, as no more data will come in.
					// This can mean tungstenite state is Terminated and we can safely drop the underlying connection.
					// Note that tungstenite only set's this on the client after the server has closed the underlying
					// connection, to comply with the RFC.
					//
					TungErr::ConnectionClosed |
					TungErr::AlreadyClosed   =>
					{
						self.state.insert( State::STREAM_CLOSED );

						self.queue_event( WsEvent::Closed );

						self.poll_next( cx )
					}


					// This generally means the underlying transport is broken. Tungstenite will keep bubbling up the
					// same error over and over, consider this fatal.
					//
					TungErr::Io(e) =>
					{
						self.state.insert( State::STREAM_CLOSED );

						self.queue_event( WsEvent::Error(Arc::new( WsErr::from( std::io::Error::from(e.kind()) ) )) );

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
					TungErr::Protocol( ref proto_err ) =>
					{
						// If this returns pending, we don't want to recurse, the task will be woken up.
						//
						ready!( self.as_mut().send_closeframe( CloseCode::Protocol, proto_err.to_string().into(), cx ) );


						self.queue_event( WsEvent::Error( Arc::new( WsErr::from(err) )) );


						// Continue to drive the event and the close handshake before returning.
						//
						self.poll_next( cx )
					}

					// This also means the remote sent a text message which isn't supported anyway, so we don't much care
					// for the utf errors
					//
					TungErr::Utf8 =>
					{
						let string = "Text messages are not supported";

						self.queue_event( WsEvent::Error( Arc::new( WsErr::from(err) )) );

						// If this returns pending, we don't want to recurse, the task will be woken up.
						//
						ready!( self.as_mut().send_closeframe( CloseCode::Unsupported, string.into(), cx ) );

						// Continue to drive the event and the close handshake before returning.
						//
						self.poll_next( cx )
					}


					// The capacity for the tungstenite read buffer is currently usize::max, and there is
					// no way for clients to change that, so this should never happen.
					//
					TungErr::Capacity(_) =>
					{
						self.queue_event( WsEvent::Error( Arc::new( WsErr::from(err) )) );
						self.poll_next( cx )
					}


					// I hope none of these can occur here because they are either handshake errors
					// or buffer capacity errors.
					//
					// This should only happen in the write side:
					//
					TungErr::WriteBufferFull(_) |

					// These are handshake errors:
					//
					TungErr::Url        (_)  |

					// I'd rather have this match exhaustive, but tungstenite has a Tls variant that
					// is only there if they have a feature enabled. Since we cannot check whether
					// a feature is enabled on a dependency, we have to go for wildcard here.
					// As of tungstenite 0.19 Http and HttpFormat are also behind a feature flag.
					//
					_ => unreachable!( "{:?}", err ),
				}
			}
		}
	}
}



impl<S> Sink<Vec<u8>> for TungWebSocket<S> where S: AsyncRead + AsyncWrite + Send + Unpin
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


		if self.state.contains( State::SINK_CLOSED )
		{
			return Err( io::ErrorKind::NotConnected.into() ).into()
		}


		Pin::new( &mut self.inner ).poll_ready( cx ).map_err( |e|
		{
			// TODO: It's not quite clear whether the stream can remain functional when we get a sink error,
			// but since this is a duplex connection, and poll_next also tries to send out close frames
			// through the stream, just consider sink errors fatal.
			//
			self.state.insert( State::STREAM_CLOSED );
			to_io_error( e )
		})
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
		if self.state.contains( State::SINK_CLOSED )
		{
			return Err( io::ErrorKind::NotConnected.into() )
		}


		Pin::new( &mut self.inner ).start_send( item.into() ).map_err( |e|
		{
			// TODO: It's not quite clear whether the stream can remain functional when we get a sink error,
			// but since this is a duplex connection, and poll_next also tries to send out close frames
			// through the stream, just consider sink errors fatal.
			//
			self.state.insert( State::STREAM_CLOSED );
			to_io_error( e )
		})
	}

	/// This will do a send under the hood, so the same errors as from start_send can occur here.
	//
	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		Pin::new( &mut self.inner ).poll_flush( cx ).map_err( |e|
		{
			// TODO: It's not quite clear whether the stream can remain functional when we get a sink error,
			// but since this is a duplex connection, and poll_next also tries to send out close frames
			// through the stream, just consider sink errors fatal.
			//
			self.state.insert( State::STREAM_CLOSED );
			to_io_error( e )
		})
	}


	/// Will resolve immediately. Keep polling the stream until it returns None. To make sure
	/// to keep the underlying connection alive until the close handshake is finished.
	///
	/// This will do a send under the hood, so the same errors as from start_send can occur here,
	/// except InvalidData.
	//
	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		self.state.insert( State::SINK_CLOSED );

		// We ignore closed errors since that's what we want, and because after calling this method
		// the sender task can in any case be dropped, and verifying that the connection can actually
		// be closed should be done through the reader task.
		//
		Pin::new( &mut self.inner ).poll_close( cx ).map_err( |e|
		{
			// TODO: It's not quite clear whether the stream can remain functional when we get a sink error,
			// but since this is a duplex connection, and poll_next also tries to send out close frames
			// through the stream, just consider sink errors fatal.
			//
			self.state.insert( State::STREAM_CLOSED );
			to_io_error( e )
		})
	}
}



// Convert tungstenite errors that can happen during sending into io::Error.
//
fn to_io_error( err: TungErr ) -> io::Error
{
	// See the wildcard at the bottom for why we need this.
	//
	#[ allow( unreachable_patterns, clippy::wildcard_in_or_patterns )]
	//
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
		// AFAICT the only one you can trigger on send is SendAfterClose unless you create control
		// frames yourself, which we don't.
		//
		TungErr::Protocol(source) =>
		{
			unreachable!( "protocol error from tungstenite on send is a bug in ws_stream_tungstenite, please report at http://github.com/najamelan/ws_stream_tungstenite/issues. The error from tungstenite is {}", source );
		}


		// This can happen when we create a message bigger than max message size in tungstenite.
		//
		TungErr::Capacity(string) => io::Error::new( io::ErrorKind::InvalidData, string ),


		// This can happen if we send a message bigger than the tungstenite `max_write_buffer_len`.
		// However `WsStream` looks at the size of this buffer and only sends up to `max_write_buffer_len`
		// bytes in one message.
		//
		TungErr::WriteBufferFull(_) => unreachable!( "TungErr::WriteBufferFull" ),

		// These are handshake errors
		//
		TungErr::Url(_) => unreachable!( "TungErr::Url" ),

		// This is an error specific to Text Messages that we don't use
		//
		TungErr::Utf8 => unreachable!( "TungErr::Utf8" ),

		// I'd rather have this match exhaustive, but tungstenite has a Tls variant that
		// is only there if they have a feature enabled. Since we cannot check whether
		// a feature is enabled on a dependency, we have to go for wildcard here.
		// As of tungstenite 0.19 Http and HttpFormat are also behind a feature flag.
		//
		x => unreachable!( "unmatched tungstenite error: {x}" ),
	}
}


impl<S> Observable< WsEvent > for TungWebSocket<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	type Error = WsErr;

	fn observe( &mut self, options: ObserveConfig< WsEvent > ) -> Observe< '_, WsEvent, Self::Error >
	{
		async move
		{
			self.notifier.observe( options ).await.map_err( Into::into )

		}.boxed()
	}
}




