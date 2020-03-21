use crate::{ import::* };


/// The error type for errors happening in _ws_stream_tungstenite_.
//
#[ derive( Debug, Error )            ]
#[ non_exhaustive                    ]
#[ allow( variant_size_differences ) ]
//
pub enum WsErr
{
	/// A tungstenite error.
	//
	#[ error( "A tungstenite error happened: {source}" )]
	//
	Tungstenite
	{
		/// The underlying error.
		//
		source: tungstenite::Error
	},

	/// An error from the underlying connection.
	//
	#[ error( "An io error happened: {source}" )]
	//
	Io
	{
		/// The underlying error.
		//
		source: io::Error
	},

	/// A websocket protocol error.
	//
	#[ error( "The remote committed a websocket protocol violation." )]
	//
	Protocol,

	/// A websocket protocol error.
	//
	#[ error( "The remote sent a Text message. Only Binary messages are accepted." )]
	//
	ReceivedText,

	/// Trying to work with an connection that is closed.
	//
	#[ error( "The connection is already closed." )]
	//
	Closed,
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



impl From< pharos::Error > for WsErr
{
	fn from( source: pharos::Error ) -> WsErr
	{
		match source.kind()
		{
			pharos::ErrorKind::Closed => WsErr::Closed,
			_                         => unreachable!() ,
		}
	}
}

