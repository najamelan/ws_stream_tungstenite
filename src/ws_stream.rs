use crate::{ import::*, tung_websocket::TungWebSocket, WsEvent, WsErr };


/// Takes a [`WebSocketStream`](async_tungstenite::WebSocketStream) and implements futures 0.3 `AsyncRead`/`AsyncWrite`/`AsyncBufRead`.
///
/// Will always create an entire Websocket message from every write. Tungstenite buffers messages up to
/// `write_buffer_size` in their [`tungstenite::protocol::WebSocketConfig`]. If you want small messages to be sent out,
/// either make sure this buffer is small enough or flush the writer.
///
/// On the other hand the `max_write_buffer_size` from tokio is the maximum size we can send in one go, otherwise
/// _tungstenite_ returns an error. Our [`AsyncWrite`] implementation never sends data that exceeds this buffer or
/// `max_message_size`.
///
/// However you still must respect the `max_message_size` of the receiving end.
///
/// ## Errors
///
/// Errors returned directly are generally io errors from the underlying stream. Only fatal errors are returned in
/// band, so consider them fatal and drop the WsStream object.
///
/// Other errors are returned out of band through [_pharos_](https://crates.io/crates/pharos):
///
/// On reading, eg. `AsyncRead::poll_read`:
/// - [`WsErr::Protocol`]: The remote made a websocket protocol violation. The connection will be closed
///   gracefully indicating to the remote what went wrong. You can just keep calling `poll_read` until `None`
///   is returned.
/// - tungstenite returned a utf8 error. Pharos will return it as a Tungstenite error. This means the remote
///   send a text message, which is not supported, so the connection will be gracefully closed. You can just keep calling
///   `poll_read` until `None` is returned.
/// - [`WsErr::ReceivedText`]: This means the remote send a text message, which is not supported, so the connection will
///   be gracefully closed. You can just keep calling `poll_read` until `None` is returned.
///
/// On writing, eg. `AsyncWrite::*` all errors are fatal.
///
/// When a Protocol error is encountered during writing, it indicates that either _ws_stream_tungstenite_ or _tungstenite_ have
/// a bug so it will panic.
//
pub struct WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	inner: IoStream< TungWebSocket<S>, Vec<u8> >,
	buffer_size: usize,
}



impl<S> WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	/// Create a new WsStream.
	//
	pub fn new( inner: ATungSocket<S> ) -> Self
	{
		let c           = inner.get_config();
		let buffer_size = std::cmp::min( c.max_write_buffer_size, c.max_message_size.unwrap_or(usize::MAX) );

		Self
		{
			buffer_size,
			inner      : IoStream::new( TungWebSocket::new( inner ) ),
		}
	}
}



impl<S> fmt::Debug for WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "WsStream over Tungstenite" )
	}
}



impl<S> AsyncWrite for WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	fn poll_write( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8] ) -> Poll< io::Result<usize> >
	{
		let buffer_size = std::cmp::min(self.buffer_size, buf.len());
		AsyncWrite::poll_write( Pin::new( &mut self.inner ), cx, &buf[..buffer_size] )
	}


	fn poll_write_vectored( mut self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &[ IoSlice<'_> ] ) -> Poll< io::Result<usize> >
	{
		let mut take_size = 0;
		let mut seen_size = 0;
		let mut next = 1;

		for (i, buf) in bufs.iter().enumerate()
		{
			let len = buf.len();
			seen_size += len;

			// If this buffer does not fit entirely
			//
			if take_size + len > self.buffer_size { break; }

			take_size += len;
			next  = i+1;
		}

		// TODO: scenarios to test:
		// - 1 buffer, empty
		// - 1 buffer, smaller than self.buffer_size
		// - 1 buffer, too big
		// - several buffers, some early ones are empty
		// - several buffers, first one is smaller than self.buffer_size

		// There is no data at all
		//
		if seen_size == 0 { return Poll::Ready(Ok(0)); }

		// If the first non-empty buffer is too big, let poll_write take the right amount out of it.
		//
		if take_size == 0
		{
			return AsyncWrite::poll_write( self, cx, bufs[next-1].get(0..).expect("index 0 not to be out of bounds") );
		}

		// If we can fill from multiple buffers, we don't try to split any buffer, just take buffers as long as they
		// fit entirely.
		//
		AsyncWrite::poll_write_vectored( Pin::new( &mut self.inner ), cx, &bufs[0..next] )
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< io::Result<()> >
	{
		AsyncWrite::poll_flush( Pin::new( &mut self.inner ), cx )
	}


	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< io::Result<()> >
	{
		Pin::new( &mut self.inner ).poll_close( cx )
	}
}



#[ cfg( feature = "tokio_io" ) ]
//
#[ cfg_attr( nightly, doc(cfg( feature = "tokio_io" )) ) ]
//
impl<S> TokAsyncWrite for WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	/// Will always flush the underlying socket. Will always create an entire Websocket message from every write,
	/// so call with a sufficiently large buffer if you have performance problems. Don't call with a buffer larger
	/// than the max message size accepted by the remote endpoint.
	//
	fn poll_write( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8] ) -> Poll< io::Result<usize> >
	{
		TokAsyncWrite::poll_write( Pin::new( &mut self.inner ), cx, buf )
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< io::Result<()> >
	{
		TokAsyncWrite::poll_flush( Pin::new( &mut self.inner ), cx )
	}


	fn poll_shutdown( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< io::Result<()> >
	{
		Pin::new( &mut self.inner ).poll_close( cx )
	}
}



impl<S> AsyncRead  for WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	fn poll_read( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8] ) -> Poll< io::Result<usize> >
	{
		AsyncRead::poll_read( Pin::new( &mut self.inner), cx, buf )
	}

	fn poll_read_vectored( mut self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &mut [IoSliceMut<'_>] ) -> Poll< io::Result<usize> >
	{
		AsyncRead::poll_read_vectored( Pin::new( &mut self.inner), cx, bufs )
	}
}


#[ cfg( feature = "tokio_io" ) ]
//
#[ cfg_attr( nightly, doc(cfg( feature = "tokio_io" )) ) ]
//
impl<S> TokAsyncRead for WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	fn poll_read( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut tokio::io::ReadBuf<'_> ) -> Poll< io::Result<()> >
	{
		TokAsyncRead::poll_read( Pin::new( &mut self.inner), cx, buf )
	}
}



impl<S> AsyncBufRead for WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	fn poll_fill_buf( self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< io::Result<&[u8]> >
	{
		Pin::new( &mut self.get_mut().inner ).poll_fill_buf( cx )
	}


	fn consume( mut self: Pin<&mut Self>, amount: usize )
	{
		Pin::new( &mut self.inner ).consume( amount )
	}
}



impl<S> Observable< WsEvent > for WsStream<S> where S: AsyncRead + AsyncWrite + Send + Unpin
{
	type Error = WsErr;

	fn observe( &mut self, options: ObserveConfig< WsEvent > ) -> Observe< '_, WsEvent, Self::Error >
	{
		async move
		{
			self.inner.observe( options ).await.map_err( Into::into )

		}.boxed()
	}
}
