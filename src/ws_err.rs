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
	/// On write this means there's a bug in ws_stream_tungstenite and it will return [`std::io::ErrorKind::Other`].
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

	/// Unreachable. This shouldn't happen but we need to match pharos error and want avoid panics.
	//
	Unreachable,
}



impl std::error::Error for WsErr
{
	fn source( &self ) -> Option<&(dyn std::error::Error + 'static)>
	{
		match &self
		{
			WsErr::Tungstenite{ source } => Some(source),
			WsErr::Io         { source } => Some(source),

			WsErr::Protocol     |
			WsErr::ReceivedText |
			WsErr::Closed       |
			WsErr::Unreachable  => None
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

				write!( f, "A tungstenite error happened: {source}" ),

			WsErr::Io{ source } =>

				write!( f, "An io error happened: {source}" ),

			WsErr::Protocol =>

				write!( f, "The remote committed a websocket protocol violation." ),

			WsErr::ReceivedText =>

				write!( f, "The remote sent a Text message. Only Binary messages are accepted." ),

			WsErr::Closed =>

				write!( f, "The connection is already closed." ),

			WsErr::Unreachable =>

				write!( f, "A bug in ws_stream_tungstenite caused an error variant that should be unreachable. Please report at github.com/najamelan/ws_stream_tungstenite/issues." ),
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
			_                         => WsErr::Unreachable,
		}
	}
}

