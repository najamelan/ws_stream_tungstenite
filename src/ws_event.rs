use crate::{ import::*, Error };


/// Events that can happen on the websocket. These are returned through the stream you can obtain
/// from `WsStream::observe`. These include close, ping and pong events which can not be returned
/// through AsyncRead/AsyncWrite, and non-fatal errors.
//
#[ derive( Debug, Clone ) ]
//
pub enum WsEvent
{
	/// Non fatal error that happened on the websocket. Non-fatal here doesn't mean the websocket is still
	/// usable, but at least is still usable enough to initiate a close handshake. If we bubble up errors
	/// through AsyncRead/AsyncWrite, codecs will always return `None` on subsequent polls, which would prevent
	/// from driving the close handshake to completion. Hence they are returned out of band.
	//
	Error( Arc<Error> ),

	/// We received a close frame from the remote. Just keep polling the stream. The close handshake will be
	/// completed for you. Once the stream returns `None`, you can drop the [WsStream](crate::WsStream).
	/// This is mainly useful in order to recover the close code and reason for debugging purposes.
	//
	CloseFrame( Option< CloseFrame<'static> > ),

	/// The remote sent a Ping message. It will automatically be answered as long as you keep polling the
	/// AsyncRead. This is returned as an event in case you want to analyze the payload, since only bytes
	/// from Binary websocket messages are passed through the AsyncRead.
	//
	Ping(Vec<u8>),

	/// The remote send us a Pong. Since we never send Pings, this is a unidirectional heartbeat.
	//
	Pong(Vec<u8>),

	/// The connection is closed. Polling WsStream will return `None` on read and `io::ErrorKind::NotConnected`
	/// on write soon. It's provided here for convenience so the task listening to these events know that
	/// the connection closed.
	/// You should not see any events after this one, so you can drop the Events stream.
	//
	Closed,
}
