use crate:: { import::* };


/// A wrapper around a WebSocket provided by tungstenite.
/// The purpose of these providers is to deliver a unified interface for to higher level
/// code abstracting out different implementations of the websocket handshake and protocol.
//
pub(crate) struct TungWebSocket<S: AsyncRead01 + AsyncWrite01>
{
	sink  : Compat01As03Sink< SplitSink01  < TTungSocket<S> >, TungMessage >,
	stream: Compat01As03    < SplitStream01< TTungSocket<S> >              >,
}


impl<S: AsyncRead01 + AsyncWrite01> TungWebSocket<S>
{
	/// Create a new Wrapper for a WebSocket provided by Tungstenite
	//
	pub(crate) fn new( inner: TTungSocket<S> ) -> Self
	{
		let (tx, rx) = inner.split();

		Self { stream: Compat01As03::new( rx ), sink: Compat01As03Sink::new( tx ) }
	}
}



impl<S: AsyncRead01 + AsyncWrite01> Stream for TungWebSocket<S>
{
	type Item = Result<Vec<u8>, io::Error>;


	/// When returning an error, this will return an error from Tung. The only thing we can know about the
	/// inner Tungstenite error is the display string. There will be no error type to match. So we just return
	/// `WsErrKind::TungError` including the string.
	//
	fn poll_next( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Option<Self::Item> >
	{
		let res = ready!( Pin::new( &mut self.stream ).poll_next( cx ) );

		match res
		{
			None             => Poll::Ready( None )                  ,
			Some(Ok ( msg )) => Poll::Ready( Some(Ok( msg.into() )) ),
			Some(Err( err )) =>
			{
				// Let's see what information we can get from the debug formatting.
				//
				error!( "TungErr: {:?}", err );

				Poll::Ready(Some(Err( to_io_error(err) )))
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


	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		trace!( "TungWebSocket: poll_close" );
		Pin::new( &mut self.sink ).poll_close( cx ).map_err( |e| to_io_error(e) )
	}
}



fn to_io_error( err: TungErr ) -> io::Error
{
	error!( "{:?}", &err );

	match err
	{
		// The connection is closed normally, probably by the remote.
		//
		TungErr::Protocol(_)         => return io::Error::from( io::ErrorKind::ConnectionReset   ) ,
		TungErr::ConnectionClosed => return io::Error::from( io::ErrorKind::NotConnected      ) ,
		TungErr::AlreadyClosed    => return io::Error::from( io::ErrorKind::ConnectionAborted ) ,
		TungErr::Io(er)           => return io::Error::from( er.kind()                        ) ,
		_                                 => return io::Error::from( io::ErrorKind::Other             ) ,
	}
}
