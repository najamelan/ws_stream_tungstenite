// Test using the AsyncRead/AsyncWrite from futures 0.3
//
//
//


use
{
	ws_stream_tungstenite :: { *                                                                } ,
	futures               :: { SinkExt, executor::LocalPool, task::LocalSpawnExt, AsyncReadExt  } ,
	futures_codec         :: { LinesCodec, Framed                                               } ,
	tokio_tungstenite     :: { WebSocketStream                                                  } ,
	tungstenite           :: { protocol::Role                                                   } ,
	futures_ringbuf       :: { RingBuffer                                                       } ,

	// log           :: { * } ,
};


// This was a verification to see if we need to handle the SendQueueFull error from tungstenite,
// but since we always flush in AsyncWrite::poll_write, it will block on flush.
//
// #[ test ]
//
#[ allow( dead_code ) ]
//
fn backpressure()
{
	// flexi_logger::Logger::with_str( "futures_codec=trace, tungstenite=trace, ws_stream_tungstenite=trace, tokio=warn" ).start().expect( "flexi_logger");

	let mut pool     = LocalPool::new();
	let mut spawner  = pool.spawner();

	let mock = RingBuffer::<u8>::new( 13 );


	let client = async
	{
		let     client = WsStream::new( WebSocketStream::from_raw_socket( mock.compat(), Role::Client, None ) );
		let mut framed = Framed::new( client, LinesCodec {} );


		framed.send( "A line\n"       .to_string() ).await.expect( "Send a line" );
		framed.send( "A second line\n".to_string() ).await.expect( "Send a line" );
	};

	spawner.spawn_local( client ).expect( "spawn client" );

	pool.run();
}

