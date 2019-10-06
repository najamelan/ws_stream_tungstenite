use crate::{ import::* };


/// The error type for errors happening in `ws_stream`.
///
/// Use [`Error::kind()`] to know which kind of error happened.
//
#[ derive( Debug ) ]
//
pub struct Error
{
	pub(crate) inner: Option< Box<dyn ErrorTrait + Send + Sync> >,
	pub(crate) kind : ErrorKind,
}



/// The different kind of errors that can happen when you use the `ws_stream` API.
//
#[ derive( Debug, Clone, Copy, Eq, PartialEq ) ]
//
pub enum ErrorKind
{
	/// A tungstenite error.
	//
	Tungstenite,

	/// An error from the underlying connection.
	//
	Io,

	/// A websocket protocol error.
	//
	Protocol,

	/// A websocket protocol error.
	//
	ReceivedText,

	/// Trying to work with an connection that is closed.
	//
	Closed,

	#[ doc( hidden ) ]
	//
	__NonExhaustive__
}



impl ErrorTrait for Error
{
	fn source( &self ) -> Option< &(dyn ErrorTrait + 'static) >
	{
		self.inner.as_ref().map( |e| -> &(dyn ErrorTrait + 'static)
		{
			e.deref()
		})
	}
}




impl fmt::Display for ErrorKind
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match self
		{
			Self::Tungstenite   => fmt::Display::fmt( "A tungstenite error happened.", f ) ,
			Self::Io            => fmt::Display::fmt( "An io error happened.", f ) ,
			Self::Protocol      => fmt::Display::fmt( "The remote committed a websocket protocol violation.", f ) ,
			Self::ReceivedText  => fmt::Display::fmt( "The remote sent a Text message. Only Binary messages are allowed.", f ) ,

			_ => unreachable!(),
		}
	}
}


impl fmt::Display for Error
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



impl Error
{
	/// Allows matching on the error kind
	//
	pub fn kind( &self ) -> &ErrorKind
	{
		&self.kind
	}
}

impl From<ErrorKind> for Error
{
	fn from( kind: ErrorKind ) -> Error
	{
		Error { inner: None, kind }
	}
}



impl From< TungErr > for Error
{
	fn from( inner: TungErr ) -> Error
	{
		let kind = match inner
		{
			TungErr::Protocol(_) => ErrorKind::Protocol    ,
			_                    => ErrorKind::Tungstenite ,
		};

		Error { inner: Some( Box::new( inner ) ), kind }
	}
}



impl From< io::Error > for Error
{
	fn from( inner: io::Error ) -> Error
	{
		Error { inner: Some( Box::new( inner ) ), kind: ErrorKind::Io }
	}
}



impl From< pharos::Error > for Error
{
	fn from( inner: pharos::Error ) -> Error
	{
		let kind = match inner.kind()
		{
			pharos::ErrorKind::Closed => ErrorKind::Closed,
			_                         => unreachable!() ,
		};

		Error { inner: Some( Box::new( inner ) ), kind }
	}
}

