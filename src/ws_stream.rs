use crate::{ import::*, tung_websocket::TungWebSocket };


/// Takes a WebSocketStream from tokio-tungstenite and implements futures 0.3 AsyncRead/AsyncWrite.
/// Please look at the documentation of the impls for those traits below for details (rustdoc will
/// collapse them).
//
pub struct WsStream<S: AsyncRead01 + AsyncWrite01>
{
	stream : IntoAsyncRead<SplitStream< TungWebSocket<S> >>,
	sink   : SplitSink  < TungWebSocket<S>, Vec<u8> > ,
}


impl<S: AsyncRead01 + AsyncWrite01> WsStream<S>
{
	/// Create a new WsStream.
	//
	pub fn new( inner: TTungSocket<S> ) -> Self
	{
		let ours = TungWebSocket::new( inner );

		let (tx, rx) = ours.split();

		Self
		{
			stream: rx.into_async_read(),
			sink  : tx                  ,
		}
	}
}



impl<S: AsyncRead01 + AsyncWrite01> fmt::Debug for WsStream<S>
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
impl<S: AsyncRead01 + AsyncWrite01> AsyncWrite for WsStream<S>
{
	/// Will always flush the underlying socket. Will always create an entire Websocket message from every write,
	/// so call with a sufficiently large buffer if you have performance problems.
	//
	fn poll_write( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8] ) -> Poll< io::Result<usize> >
	{
		trace!( "{:?}: AsyncWrite - poll_write", self );

		let res = ready!( Pin::new( &mut self.sink ).poll_ready(cx) );

		if let Err( e ) = res
		{
			trace!( "{:?}: AsyncWrite - poll_write SINK not READY", self );

			return Poll::Ready(Err( e ))
		}


		// FIXME: avoid extra copy?
		// would require a different signature of both AsyncWrite and Tungstenite (Bytes from bytes crate for example)
		//
		match Pin::new( &mut self.sink ).start_send( buf.into() )
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
				let _ = Pin::new( &mut self.sink ).poll_flush( cx );

				trace!( "{:?}: AsyncWrite - poll_write, wrote {} bytes", self, buf.len() );

				Poll::Ready(Ok ( buf.len() ))
			}

			Err(e) => Poll::Ready(Err( e )),
		}
	}


	fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll< io::Result<()> >
	{
		trace!( "{:?}: AsyncWrite - poll_flush", self );

		match ready!( Pin::new( &mut self.sink ).poll_flush(cx) )
		{
			Ok (_) => Poll::Ready(Ok ( () )) ,
			Err(e) => Poll::Ready(Err( e  )) ,
		}
	}


	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< io::Result<()> >
	{
		trace!( "{:?}: AsyncWrite - poll_close", self );

		ready!( Pin::new( &mut self.sink ).poll_close( cx ) )

			.map_err( |e|
			{
				error!( "{}", e );
				e
			})

			.into()
	}
}



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
impl<S: AsyncRead01 + AsyncWrite01> AsyncRead  for WsStream<S>
{
	fn poll_read( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8] ) -> Poll< io::Result<usize> >
	{
		Pin::new( &mut self.stream ).poll_read( cx, buf )
	}
}

