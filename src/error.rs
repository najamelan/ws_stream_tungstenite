use crate::{ import::* };


/// The error type for errors happening in `ws_stream`.
///
/// Use [`WsErr::kind()`] to know which kind of error happened.
//
#[ derive( Debug ) ]
//
pub struct WsErr
{
	pub(crate) inner: Option< Box<dyn StdError + Send> >,
	pub(crate) kind : WsErrKind,
}



/// The different kind of errors that can happen when you use the `ws_stream` API.
//
#[ derive( Debug ) ]
//
pub enum WsErrKind
{
	/// This is an error from tokio-tungstenite.
	//
	WsHandshake,

	/// An error happend on the tcp level when connecting. This will contain an inner
	/// error that you can obtain with `error.source()` for more information. The underlying
	/// error will be formatted in the display impl.
	//
	TcpConnection,

	/// A tungstenite error.
	//
	TungErr,

	/// A warp error.
	//
	WarpErr,

	/// A websocket protocol error.
	//
	Protocol,

	#[ doc( hidden ) ]
	//
	__NonExhaustive__
}



impl StdError for WsErr
{
	fn source( &self ) -> Option< &(dyn StdError + 'static) >
	{
		self.inner.as_ref().map( |e| -> &(dyn StdError + 'static)
		{
			e.deref()
		})
	}
}




impl fmt::Display for WsErrKind
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match self
		{
			Self::WsHandshake   => fmt::Display::fmt( "The WebSocket handshake failed.", f ) ,
			Self::TcpConnection => fmt::Display::fmt( "A tcp connection error happened.", f ) ,
			Self::TungErr       => fmt::Display::fmt( "A tungstenite error happened.", f ) ,
			Self::WarpErr       => fmt::Display::fmt( "WarpErr:", f ) ,
			Self::Protocol      => fmt::Display::fmt( "The remote committed a websocket protocol violation.", f ) ,

			_ => unreachable!(),
		}
	}
}


impl fmt::Display for WsErr
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		let inner = match self.source()
		{
			Some(e) => format!( " Caused by: {}", e ),
			None    => String::new()              ,
		};

		write!( f, "ws_stream::Error: {}{}", self.kind, inner )
	}
}



impl WsErr
{
	/// Allows matching on the error kind
	//
	pub fn kind( &self ) -> &WsErrKind
	{
		&self.kind
	}
}

impl From<WsErrKind> for WsErr
{
	fn from( kind: WsErrKind ) -> WsErr
	{
		WsErr { inner: None, kind }
	}
}



impl From< TungErr > for WsErr
{
	fn from( inner: TungErr ) -> WsErr
	{
		let kind = match inner
		{
			TungErr::Protocol(_) => WsErrKind::Protocol,
			_                    => WsErrKind::TungErr ,
		};

		WsErr { inner: Some( Box::new( inner ) ), kind }
	}
}


#[cfg( feature = "warp" )]
//
impl From< WarpErr > for WsErr
{
	fn from( inner: WarpErr ) -> WsErr
	{
		WsErr { inner: Some( Box::new( inner ) ), kind: WsErrKind::WarpErr }
	}
}


