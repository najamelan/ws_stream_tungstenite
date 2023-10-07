// This example explores how to properly close a connection.
//
use
{
	ws_stream_tungstenite  :: { *                                                          } ,
	futures                :: { TryFutureExt, StreamExt, SinkExt, join, executor::block_on } ,
	asynchronous_codec     :: { LinesCodec, Framed                                         } ,
	tokio                  :: { net::{ TcpListener }                                       } ,
	futures                :: { FutureExt, select, future::{ ok, ready }                   } ,
	async_tungstenite      :: { accept_async, tokio::{ TokioAdapter, connect_async }       } ,
	url                    :: { Url                                                        } ,
	tracing               :: { *                                                           } ,
	std                    :: { time::Duration                                             } ,
	futures_timer          :: { Delay                                                      } ,
	pin_utils              :: { pin_mut                                                    } ,
};



fn main()
{
	block_on( async
	{
		join!( server(), client() );

	});
}


// Server task.
// Let's assume we are mainly reacting to incoming requests, doing some processing and send a response.
//
async fn server()
{
	let socket = TcpListener::bind( "127.0.0.1:3012" ).await.unwrap();

	let (tcp, _peer_addr) = socket.accept().await.expect( "1 connection" );
	let s   = accept_async( TokioAdapter::new(tcp) ).await.expect( "ws handshake" );
	let ws  = WsStream::new( s );

	let (mut sink, mut stream) = Framed::new( ws, LinesCodec {} ).split();


	// Loop over incoming websocket messages. When this loop ends, we can safely drop the tcp connection.
	//
	while let Some( msg ) = stream.next().await
	{
		let msg = match msg
		{
			Err(e) =>
			{
				error!( "Error on server stream: {:?}", e );

				// if possible ws_stream will try to close the connection with a clean handshake,
				// but if the error persists, it will just give up and return None next iteration,
				// after which we should drop the connection.
				//
				continue;
			}

			Ok(m) => m,
		};


		info!( "server received: {}", msg.trim() );

		// Do some actual business logic, maybe send a response. Have a look at the impl of
		// AsyncWrite for WsStream for documentation on possible errors.
		//
		match sink.send( "Response from server\n".to_string() ).await
		{
			Ok(_) => {}

			Err(e) =>
			{
				error!( "Server error happend on send: {}", e );

				break;
			}
		}

		// If we decide to disconnect, we could close `sink`. That will initiate
		// the close handshake. We can continue processing incoming data until the client acknowledges
		// the close frame, after which this loop will receive None and stop. Note that we should not
		// send any more messages after we call sink.close.
		//
		// Once that happens it's safe to drop the underlying connection.
		//
		// debug!( "close server side" );
		// sink.close().await.expect( "close out" );

		// If the remote decides to close, the websocket close handshake will be handled
		// automatically for you and the stream will return None once that's finished, so you can
		// drop the connection.
	}

	// This allows us to test that the client doesn't hang if the server doesn't close the connection
	// in a timely matter. When uncommenting this line you will see the order of shutdown reverse.
	//
	Delay::new( Duration::from_secs(3) ).await;

	info!( "server end" );
}


// We make requests to the server and receive responses.
//
async fn client()
{
	let url    = Url::parse( "ws://127.0.0.1:3012" ).unwrap();
	let socket = ok( url ).and_then( connect_async ).await.expect( "ws handshake" );
	let ws     = WsStream::new( socket.0 );


	// This is a bit unfortunate, but the websocket specs say that the server should
	// close the underlying tcp connection. Hovever, since are the client, we want to
	// use a timeout just in case the server does not close the connection after a reasonable
	// amount of time. Thus when we want to initiate the close, we need to keep polling
	// the stream to drive the close handshake to completion, but we also want to be able
	// to cancel waiting on the stream if it takes to long. Thus we need to break from our
	// normal processing loop and thus need to mark here whether we broke because we want
	// to close or because the server has already closed and the stream already returned
	// None.
	//
	let mut our_shutdown = false;

	let (mut sink, mut stream) = Framed::new( ws, LinesCodec {} ).split();


	// Do some actual business logic
	// This could run in a separate task.
	//
	sink.send( "Hi from client\n".to_string() ).await.expect( "send request" );


	while let Some( msg ) = stream.next().await
	{
		let msg = match msg
		{
			Err(e) =>
			{
				error!( "Error on client stream: {:?}", e );

				// if possible ws_stream will try to close the connection with a clean handshake,
				// but if the error persists, it will just give up and return None next iteration,
				// after which we should drop the connection.
				//
				continue;
			}

			Ok(m) => m,
		};


		info!( "client received: {}", msg.trim() );

		// At some point client decides to disconnect. We will still have to poll the stream
		// to be sure the close handshake get's driven to completion, but we need to timeout
		// if the server never closes the connection. So we need to break from this loop and
		// notify the following code that the break was because we close and not because it
		// returned None.
		//
		debug!( "close client side" );
		sink.close().await.expect( "close out" );
		our_shutdown = true;
		break;
	}


	// This takes care of the timeout
	//
	if our_shutdown
	{
		// We want a future that ends when the stream ends, and that polls it in the mean time.
		// We don't want to consider any more messages from the server though.
		//
		let stream_end = stream.for_each( |_| ready(()) ).fuse();
		let timeout    = Delay::new( Duration::from_secs(1) ).fuse();

		pin_mut!( timeout    );
		pin_mut!( stream_end );

		// select over timer and end of stream
		//
		select!
		{
			_ = timeout    => {}
			_ = stream_end => {}
		}
	}

	info!( "client end" );
}






