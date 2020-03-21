use crate::{ import::*, tung_websocket::TungWebSocket, WsEvent, WsErr };


/// Takes a [`WebSocketStream`](async_tungstenite::WebSocketStream) and implements futures 0.3 `AsyncRead`/`AsyncWrite`/`AsyncBufRead`.
///
/// ## Errors
///
/// Errors returned directly are generally io errors from the underlying stream. Only fatal errors are returned in
/// band, so consider them fatal and drop the WsStream object.
///
/// Other errors are returned out of band through _pharos_:
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
/// On writing, eg. `AsyncWrite::*` all errors are fatal. Note that if you get `io::ErrorKind::InvalidData`, it means you
/// send data that exceeds the tungstenite Capacity. Eg. it leads to a message that exceeds the max message size in tungstenite.
/// It's intended that _ws_stream_tungstenite_ protects from this by splitting the data in several messages, but for now
/// there is no way to find out what the max message size is from the underlying WebSocketStream, so that will require
/// changes in the API of _tungstenite_ and _async-tungstenite_, so that will be for a future release.
///
/// When a Protocol error is encountered during writing, it indicates that either _ws_stream_tungstenite_ or _tungstenite_ have
/// a bug so it will panic.
//
pub struct WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	inner: IoStream< TungWebSocket<S>, Vec<u8> >,
}



impl<S> WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	/// Create a new WsStream.
	//
	pub fn new( inner: ATungSocket<S> ) -> Self
	{
		Self
		{
			inner : IoStream::new( TungWebSocket::new( inner ) ),
		}
	}
}



impl<S> fmt::Debug for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "WsStream over Tungstenite" )
	}
}



impl<S> AsyncWrite for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	/// Will always flush the underlying socket. Will always create an entire Websocket message from every write,
	/// so call with a sufficiently large buffer if you have performance problems. Don't call with a buffer larger
	/// than the max message size set in tungstenite (64MiB) by default.
	//
	fn poll_write( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8] ) -> Poll< io::Result<usize> >
	{
		AsyncWrite::poll_write( Pin::new( &mut self.inner ), cx, buf )
	}


	/// Will always flush the underlying socket. Will always create an entire Websocket message from every write,
	/// so call with a sufficiently large buffers if you have performance problems. Don't call with a total buffer size
	/// larger than the max message size set in tungstenite (64MiB) by default.
	//
	fn poll_write_vectored( mut self: Pin<&mut Self>, cx: &mut Context<'_>, bufs: &[ IoSlice<'_> ] ) -> Poll< io::Result<usize> >
	{
		AsyncWrite::poll_write_vectored( Pin::new( &mut self.inner ), cx, bufs )
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
impl<S> TokAsyncWrite for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
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



impl<S> AsyncRead  for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
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
impl<S> TokAsyncRead for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	fn poll_read( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8] ) -> Poll< io::Result<usize> >
	{
		TokAsyncRead::poll_read( Pin::new( &mut self.inner), cx, buf )
	}
}



impl<S> AsyncBufRead for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
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



impl<S> Observable< WsEvent > for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	type Error = WsErr;

	fn observe( &mut self, options: ObserveConfig< WsEvent > ) -> Result< Events< WsEvent >, Self::Error >
	{
		self.inner.observe( options ).map_err( Into::into )
	}
}
