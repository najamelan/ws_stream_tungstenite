use crate::{ import::* };


/// The error type for errors happening in _ws_stream_tungstenite_.
//
#[ derive( Debug )            ]
#[ non_exhaustive                    ]
#[ allow( variant_size_differences ) ]
//
pub enum WsErr
{
	/// A tungstenite error.
	//
	Tungstenite
	{
		/// The underlying error.
		//
		source: tungstenite::Error
	},

	/// An error from the underlying connection.
	//
	Io
	{
		/// The underlying error.
		//
		source: io::Error
	},

	/// A websocket protocol error. On read it means the remote didn't respect the websocket protocol.
	/// On write this means there's a bug in ws_stream_tungstenite and it will panic.
	//
	Protocol,

	/// We received a websocket text message. As we are about turning the websocket connection into a
	/// bytestream, this is probably unintended, and thus unsupported.
	//
	ReceivedText,

	/// Trying to work with an connection that is closed. Only happens on writing. On reading
	/// `poll_read` will just return `None`.
	//
	Closed,
}



impl std::error::Error for WsErr
{
	fn source( &self ) -> Option<&(dyn std::error::Error + 'static)>
	{
		match &self
		{
			WsErr::Tungstenite{ ref source } => Some(source),
			WsErr::Io         { ref source } => Some(source),

			WsErr::Protocol     |
			WsErr::ReceivedText |
			WsErr::Closed       => None
		}
	}
}



impl fmt::Display for WsErr
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match &self
		{
			WsErr::Tungstenite{ source } =>

				write!( f, "A tungstenite error happened: {}", source ),

			WsErr::Io{ source } =>

				write!( f, "An io error happened: {}", source ),

			WsErr::Protocol =>

				write!( f, "The remote committed a websocket protocol violation." ),

			WsErr::ReceivedText =>

				write!( f, "The remote sent a Text message. Only Binary messages are accepted." ),

			WsErr::Closed =>

				write!( f, "The connection is already closed." ),
		}
	}
}



impl From< TungErr > for WsErr
{
	fn from( inner: TungErr ) -> WsErr
	{
		match inner
		{
			TungErr::Protocol(_) => WsErr::Protocol              ,
			source               => WsErr::Tungstenite{ source } ,
		}
	}
}



impl From< io::Error > for WsErr
{
	fn from( source: io::Error ) -> WsErr
	{
		WsErr::Io { source }
	}
}



impl From< PharErr > for WsErr
{
	fn from( source: PharErr ) -> WsErr
	{
		match source.kind()
		{
			pharos::ErrorKind::Closed => WsErr::Closed,
			_                         => unreachable!() ,
		}
	}
}

