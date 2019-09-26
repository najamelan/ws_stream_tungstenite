use crate:: { import::*, WsEvent, Error };


// Keep track of events we need to send. They get put in a queue and on each read/write operation
// we first try to notify all observers before polling the inner stream.
//
#[ derive( Debug, Clone, Copy ) ]
//
enum State
{
	Ready,

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
	pharos      : Pharos< WsEvent > ,
	state       : State             ,
	pharos_queue: VecDeque<WsEvent> ,
}


impl Notifier
{
	pub(crate) fn new() -> Self
	{
		Self
		{
			pharos      : Pharos::default(),
			state       : State::Ready     ,
			pharos_queue: VecDeque::new()  ,
		}
	}


	pub(crate) fn queue( &mut self, evt: WsEvent )
	{
		// It should only happen if we call close on it, and we should never do that.
		//
		debug_assert!( self.state != State::Closed );

		self.pharos_queue.push_back( evt );

		self.state = State::Pending;
	}


	pub(crate) fn run( &mut self, cx: &mut Context<'_> ) -> Poll< Result<(), ()> >
	{
		debug!( "in notify" );
		let mut pharos = Pin::new( &mut self.pharos );


		match self.state
		{
			State::Ready  => Ok(()).into(),
			State::Closed => Err(()).into(),

			State::Pending =>
			{
				while let Some( evt ) = self.pharos_queue.pop_front()
				{
					match ready!( pharos.as_mut().poll_ready( cx ) )
					{
						Err(_e) =>
						{
							self.state = State::Closed;
							return Err(()).into();
						}

						Ok(()) =>
						{
							warn!( "start_send evt to pharos" );

							if let Err(_e) = pharos.as_mut().start_send( evt )
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
									warn!( "PhState::Flushing" );

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
					Err(_e) =>
					{
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
