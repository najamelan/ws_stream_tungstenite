
// Tested:
//
// ✔ do not accept a second close frame
//
use crate :: { import::*, tung_websocket::{ closer::Closer } };


#[ test ]
//
fn no_double_close()
{
	// flexi_logger::Logger::with_str( "send_text_backpressure=trace, tungstenite=trace, tokio_tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let test = async
	{
		let mut closer = Closer::new();

		closer.queue( CloseFrame{ code: CloseCode::Unsupported, reason: "tada"  .into() } ).expect( "queue close" );

		assert!( closer.queue( CloseFrame{ code: CloseCode::Unsupported, reason: "second".into() } ).is_err() );

		trace!( "server: drop websocket" );
	};

	block_on( test );
	info!( "end test" );
}




