#![cfg_attr( nightly, feature(doc_cfg) )]
#![cfg_attr( nightly, cfg_attr( nightly, doc = include_str!("../README.md") ))]
#![doc = ""] // empty doc line to handle missing doc warning when the feature is missing.

#![ doc    ( html_root_url = "https://docs.rs/ws_stream_tungstenite" ) ]
#![ deny   ( missing_docs                                            ) ]
#![ forbid ( unsafe_code                                             ) ]
#![ allow  ( clippy::suspicious_else_formatting                      ) ]

#![ warn
(
	missing_debug_implementations ,
	missing_docs                  ,
	nonstandard_style             ,
	rust_2018_idioms              ,
	trivial_casts                 ,
	trivial_numeric_casts         ,
	unused_extern_crates          ,
	unused_qualifications         ,
	single_use_lifetimes          ,
	unreachable_pub               ,
	variant_size_differences      ,
)]


mod ws_stream ;
mod ws_event  ;
mod ws_err    ;

pub(crate) mod tung_websocket;

pub use
{
	self::ws_stream :: { WsStream } ,
	self::ws_event  :: { WsEvent  } ,
	self::ws_err    :: { WsErr    } ,
};



mod import
{
	pub(crate) use
	{
		bitflags          :: { bitflags                                                                                     } ,
		futures_core      :: { ready, Stream                                                                                } ,
		futures_sink      :: { Sink                                                                                         } ,
		futures_io        :: { AsyncRead, AsyncWrite, AsyncBufRead                                                          } ,
		futures_util      :: { FutureExt                                                                                    } ,
		log               :: { error                                                                                        } ,
		std               :: { io, io::{ IoSlice, IoSliceMut }, pin::Pin, fmt, borrow::Cow                                  } ,
		std               :: { collections::VecDeque, sync::Arc, task::{ Context, Poll }                                    } ,
		async_tungstenite :: { WebSocketStream as ATungSocket                                                               } ,
		tungstenite       :: { Message as TungMessage, Error as TungErr, protocol::{ CloseFrame, frame::coding::CloseCode } } ,
		pharos            :: { Observable, ObserveConfig, Observe, Pharos, PharErr                                          } ,
		async_io_stream   :: { IoStream                                                                                     } ,
	};



	#[ cfg( feature = "tokio" ) ]
	//
	pub(crate) use
	{
		tokio::io::{ AsyncRead as TokAsyncRead, AsyncWrite as TokAsyncWrite },
	};



	#[ cfg( test ) ]
	//
	pub(crate) use
	{
		futures           :: { executor::block_on, SinkExt, StreamExt } ,
		futures_test      :: { task::noop_waker                       } ,
		pharos            :: { Channel                                } ,
		assert_matches    :: { assert_matches                         } ,
		futures_ringbuf   :: { Endpoint                               } ,
		futures           :: { future::{ join }                       } ,
		tungstenite       :: { protocol::{ Role }                     } ,
		log               :: { *                                      } ,
	};
}

