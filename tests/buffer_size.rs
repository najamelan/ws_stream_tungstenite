#![allow(unused_imports)]

// Verify the correct error is returned when sending a text message.
//
use
{
	ws_stream_tungstenite :: { *                                                                                  } ,
	std                   :: { future::Future                                                                     } ,
	futures               :: { StreamExt, SinkExt, executor::block_on, future::join                               } ,
	asynchronous_codec    :: { LinesCodec, Framed                                                                 } ,
	async_tungstenite     :: { WebSocketStream                                                                    } ,
	tungstenite           :: { protocol::{ WebSocketConfig, CloseFrame, frame::coding::CloseCode, Role }, Message } ,
	pharos                :: { Observable, ObserveConfig                                                          } ,
	assert_matches        :: { assert_matches                                                                     } ,
	async_progress        :: { Progress                                                                           } ,
	futures_ringbuf       :: { Endpoint                                                                           } ,
	tracing               :: { *                                                                                  } ,
};


// Test tungstenite buffer implementation with async_tungstenite:
// 1. fill up part of `write_buffer_size`
// 2. send a second message. The combination of both messages exceeds `max_write_buffer_size`.
// 3. verify that it gives proper backpressure and we don't get an error for exceeding the max buffer size.
//
#[ test ]
//
fn buffer_size()
{
	let (sc, cs) = Endpoint::pair( 50, 50 );

	let server = server( sc );
	let client = client( cs );

	block_on( join( server, client ) );
	info!( "end test" );
}


async fn server( sc: Endpoint )
{
	let conf = WebSocketConfig
	{
		write_buffer_size: 6,
		max_write_buffer_size: 8,
		..Default::default()
	};

	let mut tws = WebSocketStream::from_raw_socket( sc, Role::Server, Some(conf) ).await;
	let msg = Message::Binary(Vec::from("hello".as_bytes()));

	let writer = async
	{
		info!( "start sending first message" );

		tws.send( msg.clone() ).await.expect( "Send first line" );
		info!( "finished sending first message" );

		tws.send( msg ).await.expect( "Send first line" );
		info!( "finished sending second message" );

		tws.close(None).await.expect("close sink");
		trace!( "server: writer end" );
	};

	writer.await;

	trace!( "server: drop websocket" );
}



async fn client( cs: Endpoint )
{
	let     conf = WebSocketConfig::default();
	let mut ws   = WebSocketStream::from_raw_socket( cs, Role::Client, Some(conf) ).await;

	info!( "wait for client_read" );

	let test = ws.next().await.unwrap().unwrap();
	assert_matches!( test, Message::Binary(x) if x == "hello".as_bytes() );

	info!( "client: received first binary message" );

	let test = ws.next().await.unwrap().unwrap();
	assert_matches!( test, Message::Binary(x) if x == "hello".as_bytes() );

	info!( "client: received second binary message" );

	let close = ws.next().await;
	assert_matches!( close, Some(Ok(Message::Close(None))));

	trace!( "client: drop websocket" );
}
