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


mod ws_stream ;
mod ws_event  ;
mod error     ;

pub(crate) mod tung_websocket;

pub use
{
	self::ws_stream :: { WsStream         } ,
	self::ws_event  :: { WsEvent          } ,
	self::error     :: { Error, ErrorKind } ,
};



mod import
{
	pub(crate) use
	{
		bitflags          :: { bitflags                                                                                     } ,
		futures_01        :: { stream::{ SplitStream as SplitStream01, SplitSink as SplitSink01, Stream as Stream01 }       } ,
		futures::compat   :: { Compat01As03, Compat01As03Sink                                                               } ,
		futures           :: { prelude::{ Stream, Sink, AsyncRead, AsyncWrite }, Poll, task::Context, ready                 } ,
		log               :: { trace, debug, error, warn                                                                    } ,
		std               :: { io::{ self }, pin::Pin, fmt, borrow::Cow, error::Error as ErrorTrait, ops::Deref, collections::VecDeque, sync::Arc             } ,
		tokio             :: { io::{ AsyncRead as AsyncRead01, AsyncWrite as AsyncWrite01 }                                 } ,
		tokio_tungstenite :: { WebSocketStream as TTungSocket                                                               } ,
		tungstenite       :: { Message as TungMessage, Error as TungErr, protocol::{ CloseFrame, frame::coding::CloseCode } } ,
		pharos            :: { Observable, ObserveConfig, Events, Pharos                                                    } ,
	};
}

