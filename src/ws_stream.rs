use crate::{ import::*, tung_websocket::TungWebSocket, WsEvent, Error };


/// Takes a WebSocketStream from tokio-tungstenite and implements futures 0.3 AsyncRead/AsyncWrite.
/// Please look at the documentation of the impls for those traits below for details (rustdoc will
/// collapse them).
//
pub struct WsStream<S: AsyncRead + AsyncWrite + Unpin>
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
	/// so call with a sufficiently large buffer if you have performance problems.
	//
	fn poll_write( mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8] ) -> Poll< io::Result<usize> >
	{
		AsyncWrite::poll_write( Pin::new( &mut self.inner ), cx, buf )
	}


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


impl<S> Observable< WsEvent > for WsStream<S> where S: AsyncRead + AsyncWrite + Unpin
{
	type Error = Error;

	fn observe( &mut self, options: ObserveConfig< WsEvent > ) -> Result< Events< WsEvent >, Self::Error >
	{
		self.inner.observe( options ).map_err( Into::into )
	}
}
