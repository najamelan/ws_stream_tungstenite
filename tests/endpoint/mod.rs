use
{
	std        :: { io, fmt, sync::{ Arc, Mutex, atomic::{ AtomicBool, Ordering::SeqCst } } } ,
	futures_01 :: { Async, task::{ self, Task as Task01 }                                   } ,
	ringbuf    :: { RingBuffer, Producer, Consumer                                          } ,
	tokio      :: { io::{ AsyncRead as AsyncRead01, AsyncWrite as AsyncWrite01 }            } ,
	log        :: { *                                                                       } ,
};


// A mock duplex network stream implementing tokio_01::AsyncRead/Write
//
pub struct Endpoint
{
	name        : &'static str                 ,

	writer      : Producer<u8>                 ,
	reader      : Consumer<u8>                 ,

	own_rtask   : Arc<Mutex< Option<Task01> >> ,
	other_rtask : Arc<Mutex< Option<Task01> >> ,
	own_wtask   : Arc<Mutex< Option<Task01> >> ,
	other_wtask : Arc<Mutex< Option<Task01> >> ,

	own_open    : Arc<AtomicBool>              ,
	other_open  : Arc<AtomicBool>              ,
}


impl Endpoint
{
	/// Create a pair of endpoints, specifying the buffer size for each one. The buffer size corresponds
	/// to the buffer the respective endpoint writes to. The other will read from this one.
	//
	pub fn pair( a_buf: usize, b_buf: usize ) -> (Endpoint, Endpoint)
	{
		let ab_buf = RingBuffer::<u8>::new( a_buf );
		let ba_buf = RingBuffer::<u8>::new( b_buf );

		let (ab_writer, ab_reader) = ab_buf.split();
		let (ba_writer, ba_reader) = ba_buf.split();

		let a_rtask = Arc::new(Mutex::new( None ));
		let a_wtask = Arc::new(Mutex::new( None ));
		let b_rtask = Arc::new(Mutex::new( None ));
		let b_wtask = Arc::new(Mutex::new( None ));

		let a_open  = Arc::new(AtomicBool::new( true ));
		let b_open  = Arc::new(AtomicBool::new( true ));

		(
			Endpoint
			{
				name        : "A_Server"      ,
				writer      : ab_writer       ,
				reader      : ba_reader       ,
				own_rtask   : a_rtask.clone() ,
				other_rtask : b_rtask.clone() ,
				own_wtask   : a_wtask.clone() ,
				other_wtask : b_wtask.clone() ,

				own_open    : a_open.clone()  ,
				other_open  : b_open.clone()  ,
			},

			Endpoint
			{
				name        : "B_Client"     ,
				writer      : ba_writer      ,
				reader      : ab_reader      ,
				own_rtask   : b_rtask        ,
				other_rtask : a_rtask        ,
				own_wtask   : b_wtask        ,
				other_wtask : a_wtask        ,

				own_open    : b_open.clone() ,
				other_open  : a_open.clone() ,
			}
		)
	}
}



impl io::Read for Endpoint
{
	fn read( &mut self, buf: &mut [u8] ) -> io::Result<usize>
	{
		if !self.own_open.load( SeqCst )
		{
			return Ok(0);
		}

		let res = match self.reader.read( buf )
		{
			Ok(n)  =>
			{
				trace!( "{} - read {} bytes", self.name, n );

				Ok( n )
			}

			Err(e) =>
			{
				match e.kind()
				{
					io::ErrorKind::WouldBlock =>
					{
						if !self.other_open.load( SeqCst )
						{
							return Ok(0);
						}

						trace!( "{} - read: wouldblock", self.name );

						let mut own_rtask = self.own_rtask.lock().expect( "lock" );

						if own_rtask.is_some()
						{
							trace!( "{} - read: overwriting reader task", self.name );
						}

						*own_rtask = Some( task::current() );
						Err(e)
					}

					_ => Err( e )
				}
			}
		};

		if let Some( t ) = self.other_wtask.lock().expect( "lock" ).take()
		{
			trace!( "{} - read: waking up writer", self.name );
			t.notify();
		}

		else
		{
			trace!( "{} - read: no writer to wake up", self.name );
		}

		res
	}
}



impl io::Write for Endpoint
{
	fn write( &mut self, buf: &[u8] ) -> io::Result<usize>
	{
		if !self.  own_open.load( SeqCst )
		|| !self.other_open.load( SeqCst )
		{
			return Ok(0);
		}

		let res = match self.writer.write( buf )
		{
			Ok(n)  =>
			{
				trace!( "{} - wrote {} bytes", self.name, n );

				Ok( n )
			}

			Err(e) =>
			{
				match e.kind()
				{
					io::ErrorKind::WouldBlock =>
					{
						trace!( "{} - write: wouldblock", self.name );

						let mut own_wtask = self.own_wtask.lock().expect( "lock" );

						if own_wtask.is_some()
						{
							trace!( "{} - write: overwriting writer task", self.name );
						}

						*own_wtask = Some( task::current() );
						Err(e)
					}

					_ => Err( e ),
				}
			}
		};

		if let Some( t ) = self.other_rtask.lock().expect( "lock" ).take()
		{
			trace!( "{} - write: waking up reader", self.name );
			t.notify();
		}

		else
		{
			trace!( "{} - write: no reader to wake up", self.name );
		}

		res
	}


	fn flush( &mut self ) -> io::Result<()>
	{
		let res = match self.writer.flush()
		{
			Ok(_)  =>
			{
				trace!( "{} - writer flush Ok", self.name );

				Ok(())
			}

			Err(e) =>
			{
				match e.kind()
				{
					io::ErrorKind::WouldBlock =>
					{
						trace!( "{} - flush: wouldblock", self.name );

						let mut own_wtask = self.own_wtask.lock().expect( "lock" );

						if own_wtask.is_some()
						{
							trace!( "{} - flush: overwriting writer task", self.name );
						}

						*own_wtask = Some( task::current() );
						Err(e)
					}

					_ => Err( e ),
				}
			}
		};

		if let Some( t ) = self.other_rtask.lock().expect( "lock" ).take()
		{
			trace!( "{} - flush: waking up reader", self.name );
			t.notify();
		}

		else
		{
			trace!( "{} - flush: no reader to wake up", self.name );
		}

		res
	}
}


impl Drop for Endpoint
{
	fn drop( &mut self )
	{
		warn!( "{} - drop endpoint", self.name );

		self.own_open.store( false, SeqCst );

		// The other task might still have it's consumer, so the ringbuffer
		// will wtill be around. Therefor, make sure tasks wake up, so the notice we are closed.
		//
		if let Some( t ) = self.other_rtask.lock().expect( "lock" ).take()
		{
			warn!( "{} - flush: waking up reader", self.name );
			t.notify();
		}

		if let Some( t ) = self.other_wtask.lock().expect( "lock" ).take()
		{
			warn!( "{} - flush: waking up reader", self.name );
			t.notify();
		}
	}
}


impl AsyncRead01 for Endpoint {}


impl AsyncWrite01 for Endpoint
{
	fn shutdown( &mut self ) -> io::Result< Async<()> >
	{
		self.own_open.store( false, SeqCst );

		if let Some( t ) = self.other_rtask.lock().expect( "lock" ).take()
		{
			warn!( "{} - shutdown, waking up reader", self.name );
			t.notify();
		}

		Ok( ().into() )
	}
}


impl fmt::Debug for Endpoint
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "Endpoint01 {}", self.name )
	}
}
