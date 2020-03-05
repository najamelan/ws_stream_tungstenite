use crate::{ import::*, tung_websocket::TungWebSocket, WsEvent, Error };


/// Takes a WebSocketStream from tokio-tungstenite and implements futures 0.3 AsyncRead/AsyncWrite.
/// Please look at the documentation of the impls for those traits below for details (rustdoc will
/// collapse them).
//
pub struct WsStream<S: AsyncRead + AsyncWrite + Unpin>
{
	inner : TungWebSocket<S>,
	state : ReadState,
}


impl<S: AsyncRead + AsyncWrite + Unpin> WsStream<S>
{
	/// Create a new WsStream.
	//
	pub fn new( inner: ATungSocket<S> ) -> Self
	{
		Self
		{
			inner : TungWebSocket::new( inner ),
			state : ReadState::PendingChunk
		}
	}
}



impl<S: AsyncRead + AsyncWrite + Unpin> fmt::Debug for WsStream<S>
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "WsStream over Tungstenite" )
	}
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
impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for WsStream<S>
{
	/// Will always flush the underlying socket. Will always create an entire Websocket message from every write,
	/// so call with a sufficiently large buffer if you have performance problems.
	//
	fn poll_write( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8] ) -> Poll< io::Result<usize> >
	{
		trace!( "{:?}: AsyncWrite - poll_write", self );

		let res = ready!( Pin::new( &mut self.inner ).poll_ready(cx) );

		if let Err( e ) = res
		{
			trace!( "{:?}: AsyncWrite - poll_write SINK not READY", self );

			return Poll::Ready(Err( e ))
		}


		// FIXME: avoid extra copy?
		// would require a different signature of both AsyncWrite and Tungstenite (Bytes from bytes crate for example)
		//
		match Pin::new( &mut self.inner ).start_send( buf.into() )
		{
			Ok (_) =>
			{
				// The Compat01As03Sink always keeps one item buffered. Also, client code like
				// futures-codec and tokio-codec turn a flush on their sink in a poll_write here.
				// Combinators like CopyBufInto will only call flush after their entire input
				// stream is exhausted.
				// We actually don't buffer here, but always create an entire websocket message from the
				// buffer we get in poll_write, so there is no reason not to flush here, especially
				// since the sink will always buffer one item until flushed.
				// This means the burden is on the caller to call with a buffer of sufficient size
				// to avoid perf problems, but there is BufReader and BufWriter in the futures library to
				// help with that if necessary.
				//
				// We will ignore the Pending return from the flush, since we took the data and
				// must return how many bytes we took. The client should not try to send this data again.
				// This does mean there might be a spurious wakeup, TODO: we should test that.
				// We could supply a dummy context to avoid the wakup.
				//
				// So, flush!
				//
				let _ = Pin::new( &mut self.inner ).poll_flush( cx );

				trace!( "{:?}: AsyncWrite - poll_write, wrote {} bytes", self, buf.len() );

				Poll::Ready(Ok ( buf.len() ))
			}

			Err(e) => Poll::Ready(Err( e )),
		}
	}


	fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll< io::Result<()> >
	{
		trace!( "{:?}: AsyncWrite - poll_flush", self );

		match ready!( Pin::new( &mut self.inner ).poll_flush(cx) )
		{
			Ok (_) => Poll::Ready(Ok ( () )) ,
			Err(e) => Poll::Ready(Err( e  )) ,
		}
	}


	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< io::Result<()> >
	{
		trace!( "{:?}: AsyncWrite - poll_close", self );

		ready!( Pin::new( &mut self.inner ).poll_close( cx ) ).into()
	}
}






#[derive(Debug, Clone)]
//
enum ReadState
{
	Ready { chunk: Vec<u8>, chunk_start: usize } ,
	PendingChunk                                 ,
}

/// When None is returned, it means it is safe to drop the underlying connection.
///
/// TODO: can we not just save an IntoAsyncRead on self instead of the TungWebSocket?
///       that way we don't need to duplicate the algorithm here?
///
/// ### Errors
///
/// TODO: document errors
//
impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead  for WsStream<S>
{
	fn poll_read( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8] ) -> Poll< io::Result<usize> >
	{
		trace!( "WsStream: read called" );

		loop { match &mut self.state
		{
			ReadState::Ready { chunk, chunk_start } =>
			{
				trace!( "io_read: received message" );

				let end = std::cmp::min( *chunk_start + buf.len(), chunk.len() );
				let len = end - *chunk_start;

				buf[..len].copy_from_slice( &chunk[*chunk_start..end] );


				// We read the entire chunk
				//
				if chunk.len() == end { self.state = ReadState::PendingChunk }
				else                  { *chunk_start = end                   }

				trace!( "io_read: return read {}", len );

				return Poll::Ready( Ok(len) );
			}


			ReadState::PendingChunk =>
			{
				trace!( "io_read: pending" );

				match ready!( Pin::new( &mut self.inner ).poll_next(cx) )
				{
					// We have a message
					//
					Some(Ok( chunk )) =>
					{
						self.state = ReadState::Ready { chunk, chunk_start: 0 };
						continue;
					}

					// The stream has ended
					//
					None =>
					{
						trace!( "ws_stream: stream has ended" );
						return Ok(0).into();
					}

					Some( Err(err) ) =>
					{
						error!( "{}", err );

						return Poll::Ready(Err( err ))
					}
				}
			}
		}}
	}
}


impl<S> Observable< WsEvent > for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	type Error = Error;

	fn observe( &mut self, options: ObserveConfig< WsEvent > ) -> Result< Events< WsEvent >, Self::Error >
	{
		self.inner.observe( options ).map_err( Into::into )
	}
}
