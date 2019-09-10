// See: https://github.com/rust-lang/rust/issues/44732#issuecomment-488766871
//
#![ cfg_attr( feature = "external_doc", feature(external_doc)         ) ]
#![ cfg_attr( feature = "external_doc", doc(include = "../README.md") ) ]
//!


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


mod error         ;
mod ws_stream     ;
pub(crate) mod tung_websocket;

pub use
{
	error           :: * ,

	self::ws_stream :: * ,
};



mod import
{
	pub(crate) use
	{
		futures_01        :: { stream::{ SplitStream as SplitStream01, SplitSink as SplitSink01, Stream as Stream01 } } ,
		futures::compat   :: { Compat01As03, Compat01As03Sink                                                         } ,
		futures           :: { prelude::{ Stream, Sink, AsyncRead, AsyncWrite }, Poll, task::Context, ready           } ,
		futures           :: { StreamExt, TryStreamExt, stream::{ SplitStream, SplitSink, IntoAsyncRead }             } ,
		log               :: { trace, error                                                                           } ,
		std               :: { io::{ self }, pin::Pin, fmt, error::Error as StdError, ops::Deref                      } ,
		tokio             :: { io::{ AsyncRead as AsyncRead01, AsyncWrite as AsyncWrite01 }                           } ,
		tokio_tungstenite :: { WebSocketStream as TTungSocket                                                         } ,
		tungstenite       :: { Message as TungMessage, Error as TungErr                                               } ,
	};
}

