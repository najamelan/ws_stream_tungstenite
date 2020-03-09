use crate:: { import::*, WsEvent, Error };


// The different states we can be in.
//
#[ derive( Debug, Clone, Copy ) ]
//
enum State
{
	Ready,

	// Something is in the queue
	//
	Pending,

	// Pharos is closed
	//
	Closed,

	// Pharos returns pending on flush, try flushing before sending anything else
	//
	Flushing,
}



impl PartialEq for State
{
	fn eq( &self, other: &Self ) -> bool
	{
		std::mem::discriminant( self ) == std::mem::discriminant( other )
	}
}



pub(super) struct Notifier
{
	pharos: Pharos  < WsEvent > ,
	events: VecDeque< WsEvent > ,
	state : State               ,
}



impl Notifier
{
	pub(crate) fn new() -> Self
	{
		Self
		{
			// Most of the time there will probably not be many observers
			// this keeps memory consumption down
			//
			pharos: Pharos::new( 2 ) ,
			state : State::Ready     ,
			events: VecDeque::new()  ,
		}
	}


	pub(crate) fn queue( &mut self, evt: WsEvent )
	{
		// It should only happen if we call close on it, and we should never do that.
		//
		debug_assert!( self.state != State::Closed );

		self.events.push_back( evt );

		self.state = State::Pending;
	}


	// try to send out queued events.
	//
	pub(crate) fn run( &mut self, cx: &mut Context<'_> ) -> Poll< Result<(), ()> >
	{
		let mut pharos = Pin::new( &mut self.pharos );

		match self.state
		{
			State::Ready  => Ok (()).into(),
			State::Closed => Err(()).into(),

			State::Pending =>
			{
				while !self.events.is_empty()
				{
					match ready!( pharos.as_mut().poll_ready( cx ) )
					{
						Err(e) =>
						{
							error!( "pharos returned an error, this could be a bug in ws_stream_tungstenite, please report: {:?}", e );

							self.state = State::Closed;

							return Err(()).into();
						}

						Ok(()) =>
						{
							// note we can only get here if the queue isn't empty, so unwrap
							//
							if let Err(_e) = pharos.as_mut().start_send( self.events.pop_front().expect( "pop queued event." ) )
							{
								self.state = State::Closed;

								return Err(()).into();
							}

							// Flush
							//
							match pharos.as_mut().poll_flush( cx )
							{
								Poll::Pending =>
								{
									self.state = State::Flushing;

									return Poll::Pending
								}


								Poll::Ready(Err(_e)) =>
								{
									self.state = State::Closed;

									return Err(()).into();
								}

								// We are really done
								//
								Poll::Ready(Ok(())) => {}
							}
						}
					}
				}

				self.state = State::Ready;
				Ok(()).into()
			}


			State::Flushing =>
			{
				// Flush
				//
				match ready!( pharos.as_mut().poll_flush( cx ) )
				{
					Err(e) =>
					{
						error!( "pharos returned an error, this could be a bug in ws_stream_tungstenite, please report: {:?}", e );

						self.state = State::Closed;

						return Err(()).into();
					}

					// We are really done
					//
					Ok(()) =>
					{
						self.state = State::Ready;

						Ok(()).into()
					}
				}
			}
		}
	}
}



impl Observable< WsEvent > for Notifier
{
	type Error = Error;

	fn observe( &mut self, options: ObserveConfig< WsEvent > ) -> Result< Events< WsEvent >, Self::Error >
	{
		self.pharos.observe( options ).map_err( Into::into )
	}
}



#[ cfg( test ) ]
//
mod tests
{
	// Tested:
	//
	// ✔ state gets updated correctly
	// ✔ queue get's filled up and emptied
	// ✔ verify everything get's delivered correctly after pharos gives back pressure
	//
	use super::*;


	// verify state becomes pending when queing something and get's reset after calling run without observers.
	//
	#[ test ]
	//
	fn notifier_state()
	{

		let mut not = Notifier::new();

			assert_eq!( State::Ready, not.state );


		not.queue( WsEvent::Ping( vec![ 1, 2, 3] ) );

			assert_eq!( State::Pending, not.state );


		let     w   = noop_waker();
		let mut cx  = Context::from_waker( &w );
		let     res = not.run( &mut cx );

			assert_eq!( Poll::Ready( Ok(()) ), res       );
			assert_eq!(          State::Ready, not.state );



		not.queue( WsEvent::Closed );

			assert_eq!( State::Pending, not.state );
	}


	// verify state changes using an observer that provides back pressure
	//
	#[ test ]
	//
	fn notifier_state_observers()
	{
		let mut not  = Notifier::new();
		let mut evts = not.observe( Channel::Bounded( 1 ).into() ).expect( "observe" );

			assert_eq!( State::Ready, not.state        );
			assert_eq!(            0, not.events.len() );


		// Queue 2 so the channel gives back pressure.
		//
		not.queue( WsEvent::Ping( vec![ 1, 2, 3] ) );
		not.queue( WsEvent::Ping( vec![ 1, 2, 3] ) );

			assert_eq!( State::Pending, not.state        );
			assert_eq!(              2, not.events.len() );


		// delivers 1 and blocks on back pressure
		//
		let     w   = noop_waker();
		let mut cx  = Context::from_waker( &w );
		let     res = not.run( &mut cx );

			assert_eq!(  Poll::Pending, res              );
			assert_eq!( State::Pending, not.state        );
			assert_eq!(              1, not.events.len() );


		// Make more space
		//
		let evt = block_on( evts.next() );
			assert_matches!( evt, Some( WsEvent::Ping(_) ) );


		// now there should be place for the second one
		//
		let     w   = noop_waker();
		let mut cx  = Context::from_waker( &w );
		let     res = not.run( &mut cx );

			assert_eq!( Poll::Ready( Ok(()) ), res              );
			assert_eq!(          State::Ready, not.state        );
			assert_eq!(                     0, not.events.len() );


		// read the second one
		//
		let evt = block_on( evts.next() );

			assert_matches!( evt, Some( WsEvent::Ping(_) ) );
	}



	#[ test ]
	//
	fn queue()
	{
		let mut not = Notifier::new();

			assert_eq!( 0, not.events.len() );


		not.queue( WsEvent::Ping( vec![ 1, 2, 3] ) );

			assert_eq!( 1, not.events.len() );


		let     w  = noop_waker();
		let mut cx = Context::from_waker( &w );
		let     _  = not.run( &mut cx );

			assert_eq!( 0, not.events.len() );
	}
}
