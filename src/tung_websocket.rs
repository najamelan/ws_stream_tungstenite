use crate:: { import::* };


/// A wrapper around a WebSocket provided by tungstenite.
/// The purpose of these providers is to deliver a unified interface for to higher level
/// code abstracting out different implementations of the websocket handshake and protocol.
//
pub(crate) struct TungWebSocket<S: AsyncRead01 + AsyncWrite01>
{
	sink          : Compat01As03Sink< SplitSink01  < TTungSocket<S> >, TungMessage > ,
	stream        : Compat01As03    < SplitStream01< TTungSocket<S> >              > ,
	closer        : Option<Waker>                                                    ,
	close_received: bool                                                             ,
	close_sent    : bool                                                             ,
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
			stream        : Compat01As03    ::new( rx ) ,
			sink          : Compat01As03Sink::new( tx ) ,
			closer        : None                        ,
			close_received: false                       ,
			close_sent    : false                       ,
		}
	}


	/// Wake up any task that waits for the remote to acknowledge the close frame.
	//
	fn close_received( &mut self )
	{
		self.close_received = true;

		if let Some(waker) = self.closer.take()
		{
			info!( "waking closer" );
			waker.wake();
		}
	}


	async fn close_with_code_and_reason( mut self: Pin<&mut Self>, code: CloseCode, reason: Cow<'static, str> )

		-> Result<(), io::Error>
	{
		// TODO: check close_sent?
		//
		let frame = CloseFrame
		{
			code  : code   ,
			reason: reason ,
		};

		self.close_sent = true;
		self.sink.send( TungMessage::Close(Some( frame )) ).await.map_err( |e| to_io_error(e) )
	}
}



impl<S: AsyncRead01 + AsyncWrite01> Stream for TungWebSocket<S>
{
	type Item = Result<Vec<u8>, io::Error>;


	/// Get the next websocket message and convert it to a Vec<u8>.
	///
	/// When None is returned, it means it is safe to drop the underlying connection after calling [`close`]
	/// on this object and waiting for it to resolve.
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
			None =>
			{
				// If the stream ends without sending us a close frame, we still want to wake up the
				// writer task.
				//
				self.close_received();

				Poll::Ready( None )
			}


			Some(Ok ( msg )) =>
			{
				match msg
				{
					TungMessage::Binary(vec) => Poll::Ready( Some(Ok( vec )) ),


					TungMessage::Text(_) =>
					{
						let string = "Text messages are not supported";

						let fut = self.close_with_code_and_reason( CloseCode::Protocol, string.into() );

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

						self.close_received();

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
					//
					TungErr::ConnectionClosed |
					TungErr::AlreadyClosed    => None.into(),

					TungErr::Io(e)            => Some( Err(e) ).into(),

					TungErr::Protocol(string) =>
					{
						error!( "Protocol error from Tungstenite: {}", string );

						let fut = self.close_with_code_and_reason( CloseCode::Protocol, string.clone() );

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

						let fut = self.close_with_code_and_reason( CloseCode::Unsupported, string.into() );

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
		Pin::new( &mut self.sink ).poll_ready( cx ).map_err( |e| to_io_error(e) )
	}


	fn start_send( mut self: Pin<&mut Self>, item: Vec<u8> ) -> Result<(), Self::Error>
	{
		trace!( "TungWebSocket: start_send" );
		Pin::new( &mut self.sink ).start_send( item.into() ).map_err( |e| to_io_error(e) )
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		trace!( "TungWebSocket: poll_flush" );

		Pin::new( &mut self.sink ).poll_flush( cx ).map_err( |e| to_io_error(e) )
	}


	/// Will resolve when the remote has acknowlegded the close and it is safe to drop the underlying
	/// network connection.
	//
	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		trace!( "TungWebSocket: poll_close" );

		// close_received means either we initiated it somehow, or tungstenite will have replied to it anyway, so this means
		// that we have both sent and received a close frame.
		//
		// close_sent is there because we also sometimes close the connection from the reader part when protocol errors happen.
		// It prevents us from sending close to the underlying tungstenite object twice.
		//
		if self.close_received || self.close_sent
		{
			Ok(()).into()
		}

		else
		{
			self.close_sent = true;

			match ready!( Pin::new( &mut self.sink ).poll_close( cx ).map_err( |e| to_io_error(e) ) )
			{
				Err(e ) => Err(e).into(),

				// Wait for the remote to acknowledge
				//
				Ok (()) =>
				{
					self.closer = Some( cx.waker().clone() );

					trace!( "TungWebSocket: poll_close returning Pending" );
					Poll::Pending
				}
			}
		}
	}
}



fn to_io_error( err: TungErr ) -> io::Error
{
	error!( "{:?}", &err );

	match err
	{
		TungErr::Io(err)          => err                                            ,
		TungErr::ConnectionClosed => io::Error::from( io::ErrorKind::NotConnected ) ,
		TungErr::AlreadyClosed    => io::Error::from( io::ErrorKind::NotConnected ) ,

		TungErr::Protocol(string) =>
		{
			error!( "Protocol error from Tungstenite: {}", string );
			io::Error::from( io::ErrorKind::ConnectionReset )
		}

		_ => io::Error::from( io::ErrorKind::Other ) ,
	}
}
