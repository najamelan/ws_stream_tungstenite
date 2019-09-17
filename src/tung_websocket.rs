use crate:: { import::* };


/// A wrapper around a WebSocket provided by tungstenite.
/// The purpose of these providers is to deliver a unified interface for to higher level
/// code abstracting out different implementations of the websocket handshake and protocol.
//
pub(crate) struct TungWebSocket<S: AsyncRead01 + AsyncWrite01>
{
	sink  : Compat01As03Sink< SplitSink01  < TTungSocket<S> >, TungMessage > ,
	stream: Compat01As03    < SplitStream01< TTungSocket<S> >              > ,
	broken: bool                                                             ,
}


impl<S: AsyncRead01 + AsyncWrite01> TungWebSocket<S>
{
	/// Create a new Wrapper for a WebSocket provided by Tungstenite
	//
	pub(crate) fn new( inner: TTungSocket<S> ) -> Self
	{
		let (tx, rx) = inner.split();

		Self
		{
			stream: Compat01As03    ::new( rx ) ,
			sink  : Compat01As03Sink::new( tx ) ,
			broken: false                       ,
		}
	}


	async fn close_because( mut self: Pin<&mut Self>, code: CloseCode, reason: Cow<'static, str> )

		-> Result<(), io::Error>
	{
		// TODO: check close_sent?
		//
		let frame = CloseFrame { code, reason };

		self.sink.send( TungMessage::Close(Some( frame )) ).await.map_err( to_io_error )
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
		trace!( "poll tokio-tungstenite for read" );

		let res = ready!( Pin::new( &mut self.stream ).poll_next( cx ) );

		match res
		{
			None => None.into(),

			Some(Ok ( msg )) =>
			{
				match msg
				{
					TungMessage::Binary(vec) => Poll::Ready( Some(Ok( vec )) ),


					TungMessage::Text(_) =>
					{
						let string = "Text messages are not supported";

						let fut = self.close_because( CloseCode::Unsupported, string.into() );

						pin_mut!( fut  );

						match fut.poll( cx )
						{
							Poll::Pending         => Poll::Pending,
							Poll::Ready( Err(e) ) => Some( Err(e) ).into(),
							Poll::Ready( Ok(()) ) => Some( Err( io::Error::new( io::ErrorKind::InvalidData, string ) ) ).into(),
						}
					}


					TungMessage::Close (opt) =>
					{
						debug!( "received close frame: {:?}", opt );

						// Tungstenite will keep this stream around until the underlying connection closes.
						// It's important we don't return None here so clients don't drop the underlying connection
						// while the other end is still processing stuff, otherwise they receive a connection reset
						// error and can't read any more data waiting to be processed.
						//
						self.poll_next( cx )
					}


					// Tungstenite will have answered it already
					//
					TungMessage::Ping(_) |
					TungMessage::Pong(_) => self.poll_next( cx ),
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
						None.into()
					}

					// This generally means the underlying transport is broken. Tungstenite will keep bubbling up the
					// same error over and over, which is quite a hassle to handle properly.
					//
					// So we return it once, and if a second error happens, we just return None.
					//
					TungErr::Io(e) =>
					{
						if self.broken
						{
							None.into()
						}

						else
						{
							self.broken = true;
							Some( Err(e) ).into()
						}
					}

					TungErr::Protocol(string) =>
					{
						error!( "Protocol error from Tungstenite: {}", string );

						let fut = self.close_because( CloseCode::Protocol, string.clone() );

						pin_mut!( fut  );

						match fut.poll( cx )
						{
							Poll::Pending         => Poll::Pending,
							Poll::Ready( Err(e) ) => Some( Err(e) ).into(),
							Poll::Ready( Ok(()) ) => Some( Err( io::Error::new( io::ErrorKind::InvalidData, string ) ) ).into(),
						}
					}

					// This also means the remote sent a text message which isn't supported anyway, so we don't much care
					// for the utf errors
					//
					TungErr::Utf8 =>
					{
						let string = "Text messages are not supported";

						let fut = self.close_because( CloseCode::Unsupported, string.into() );

						pin_mut!( fut  );

						match fut.poll( cx )
						{
							Poll::Pending         => Poll::Pending,
							Poll::Ready( Err(e) ) => Some( Err(e) ).into(),
							Poll::Ready( Ok(()) ) => Some( Err( io::Error::new( io::ErrorKind::InvalidData, string ) ) ).into(),
						}
					}


					// I hope none of these can occur here because they are either handshake errors
					// or buffer capacity errors.
					//
					// TODO: actually capacity is on incoming as well
					//
					TungErr::Capacity     (_) |

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



impl<S: AsyncRead01 + AsyncWrite01> Sink<Vec<u8>> for TungWebSocket<S>
{
	type Error = io::Error;


	fn poll_ready( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		Pin::new( &mut self.sink ).poll_ready( cx ).map_err( to_io_error )
	}


	/// ### Errors
	///
	/// The following errors can be returned when writing to the stream:
	///
	/// - [`io::ErrorKind::NotConnected`]: This means that the connection is already closed. You should
	///   drop it. It is safe to drop the underlying connection.
	///
	/// - [`io::ErrorKind::InvalidData`]: This means that a tungstenite::error::Capacity occurred. This means that
	///   you send in a buffer bigger than the maximum message size configured on the underlying websocket connection.
	///   If you did not set it manually, the default for tungstenite is 64MB.
	///
	/// - other std::io::Error's generally mean something went wrong on the underlying transport. Consider these fatal
	///   and just drop the connection.
	//
	fn start_send( mut self: Pin<&mut Self>, item: Vec<u8> ) -> Result<(), Self::Error>
	{
		trace!( "TungWebSocket: start_send" );
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


		// Connection is closed, does not indicate something went wrong
		//
		TungErr::ConnectionClosed => io::ErrorKind::NotConnected.into() ,
		TungErr::AlreadyClosed    => io::ErrorKind::NotConnected.into() ,


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
