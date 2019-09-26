use
{
	crate :: { import::*, WsEvent, Error } ,
	super :: { notifier::Notifier        } ,
};



// Keep track of events we need to send. They get put in a queue and on each read/write operation
// we first try to notify all observers before polling the inner stream.
//
#[ derive( Debug, Clone ) ]
//
enum State
{
	Ready,

	// When we receive protocol errors or text messages, we want to close, and we want to be able to
	// send a close frame so the remote can debug their issues.
	// However it's an async operation to send this and we are in a reader task atm, so if the sink
	// is not ready to receive more data, store it here for now and try again to do this first on each
	// read or write from the user.
	//
	Closing(CloseFrame<'static>),

	// When we are closing, and the sink says yes to poll_ready, but it says Pending to flush, we store that
	// fact, so we will continue trying to flush on subsequent operations.
	//
	Flushing,


	SinkError,

	// We have finished sending a close frame, so we shouldn't do anything anymore.
	//
	Closed,
}






impl PartialEq for State
{
	fn eq( &self, other: &Self ) -> bool
	{
		std::mem::discriminant( self ) == std::mem::discriminant( other )
	}
}


pub(super) struct Closer
{
	state: State,

}


impl Closer
{
	pub(super) fn new() -> Self
	{
		Self
		{
			state: State::Ready,
		}
	}



	pub(super) fn queue( &mut self, frame: CloseFrame<'static> )
	{
		if self.state == State::Ready
		{
			self.state = State::Closing( frame );
		}


		// TODO: else what?
	}



	// Will try to send out a close frame to the websocket. It will then poll that send for completion
	// saving it's state and returning pending if no more progress can be made.
	//
	// Any errors that happen will be returned out of band as pharos events.
	//
	pub(super) fn run
	(
		mut self   : Pin<&mut Self>                                         ,
		mut socket : impl Sink<tungstenite::Message, Error=TungErr> + Unpin ,
		    ph     : &mut Notifier                                          ,
		    cx     : &mut Context<'_>	                                      ,
	)
		-> Poll< Result<(), ()> >

	{
		debug!( "in close because" );

		match &self.state
		{
			State::Ready     => Ok(()).into(),
			State::SinkError => Err(()).into(),
			State::Closed    => Err(()).into(),

			State::Closing( frame ) =>
			{
				let ready = Pin::new( &mut socket ).as_mut().poll_ready( cx );

				match ready
				{
					Poll::Pending =>
					{
						warn!( "self.close_frame = Some( frame )" );

						Poll::Pending
					}

					Poll::Ready(Err(e)) =>
					{
						error!( "{:?}", e );

						ph.queue( WsEvent::Error( Arc::new( Error::from(e) )) );

						self.state = State::SinkError;
						Err(()).into()
					}

					Poll::Ready(Ok(())) =>
					{
						warn!( "start send of close frame" );

						// Send the frame
						//
						if let Err(e) = Pin::new( &mut socket ).as_mut().start_send( TungMessage::Close( Some(frame.clone()) ) )
						{
							error!( "{:?}", e );

							ph.queue( WsEvent::Error( Arc::new( Error::from(e) )) );

							self.state = State::SinkError;

							().into()
						}

						// Flush
						//
						match Pin::new( &mut socket ).as_mut().poll_flush( cx )
						{
							Poll::Pending =>
							{
								warn!( "TwState::Flushing" );

								self.state = State::Flushing;

								Poll::Pending
							}

							// We are really done
							//
							Poll::Ready(Ok(())) =>
							{
								self.state = State::Closed;

								Ok(()).into()
							}


							Poll::Ready(Err(e)) =>
							{
								error!( "{:?}", e );

								ph.queue( WsEvent::Error( Arc::new( Error::from(e) )) );

								self.state = State::SinkError;

								Err(()).into()
							}
						}
					}
				}
			}


			State::Flushing =>
			{
				// Flush
				//
				match Pin::new( &mut socket ).as_mut().poll_flush( cx )
				{
					Poll::Pending =>
					{
						warn!( "TwState::Flushing" );

						self.state = State::Flushing;

						Poll::Pending
					}

					// We are really done
					//
					Poll::Ready(Ok(())) =>
					{
						self.state = State::Closed;

						Err(()).into()
					}


					Poll::Ready(Err(e)) =>
					{
						ph.queue( WsEvent::Error( Arc::new( Error::from(e) )) );

						self.state = State::SinkError;

						Err(()).into()
					}
				}
			}
		}
	}
}
