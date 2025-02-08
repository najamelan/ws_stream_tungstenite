//! This is an echo server that returns all incoming bytes, without framing.
//!
//! This demonstrates how to set up an ssl websocket.
//
use
{
    async_tungstenite     :: { accept_async, tokio::{ TokioAdapter }         } ,
    futures               :: { AsyncReadExt, io::{ BufReader, copy_buf }     } ,
    std                   :: { env, net::SocketAddr, io, sync::Arc } ,
    tracing               :: { *                                             } ,
    tokio                 :: { net::{ TcpListener, TcpStream }               } ,
    tokio_rustls          :: { rustls::{ ServerConfig },TlsAcceptor          } ,
    rustls::pki_types     :: { CertificateDer, PrivateKeyDer, pem::PemObject } ,
    ws_stream_tungstenite :: { *                                             } ,
};


#[tokio::main]
//
async fn main()
{
    // flexi_logger::Logger::with_str( "echo=trace, ws_stream_tungstenite=debug, tungstenite=warn, tokio_tungstenite=warn, tokio=warn" ).start().unwrap();

    // Load self-signed cert and private key
    let certs = CertificateDer::pem_file_iter("localhost-cert.pem")
        .expect("load certs")
        .collect::<Result<Vec<_>, _>>()
        .expect("load certs2");

    let key = PrivateKeyDer::from_pem_file("localhost-key.pem")
        .expect("load key");

    // Set up Rustls server config
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("invalid key or certificate");

    let acceptor = TlsAcceptor::from(Arc::new(config));

    let addr: SocketAddr = env::args().nth(1).unwrap_or_else( || "127.0.0.1:8443".to_string() ).parse().unwrap();
    println!( "server task listening at: {}", &addr );

    let socket = TcpListener::bind(&addr).await.unwrap();

    loop
    {
        let stream = socket.accept().await;
        let task   = handle_conn( stream, acceptor.clone() );
        tokio::spawn( task );
    }
}


async fn handle_conn( stream: Result< (TcpStream, SocketAddr), io::Error>, acceptor: TlsAcceptor )
{
    // If the TCP stream fails, we stop processing this connection
    //
    let (tcp_stream, peer_addr) = match stream
    {
        Ok( tuple ) => tuple,

        Err(e) =>
        {
            debug!( "Failed TCP incoming connection: {}", e );
            return;
        }
    };

    info!( "Incoming connection from: {}", peer_addr );

    let tls_stream = match acceptor.accept(tcp_stream).await
    {
        Ok(s) => s,

        Err(e) => {
            eprintln!("TLS accept error from {}: {}", peer_addr, e);
            return;
        }
    };

    println!("TLS handshake successful: {}", peer_addr);

    let s = accept_async( TokioAdapter::new(tls_stream) ).await;


    // If the Ws handshake fails, we stop processing this connection
    //
    let socket = match s
    {
        Ok(ws) => ws,

        Err(e) =>
        {
            debug!( "Failed WebSocket HandShake: {}", e );
            return;
        }
    };


    let ws_stream = WsStream::new( socket );
    let (reader, mut writer) = ws_stream.split();

    // BufReader allows our AsyncRead to work with a bigger buffer than the default 8k.
    // This improves performance quite a bit.
    //
    if let Err(e) = copy_buf( BufReader::with_capacity( 64_000, reader ), &mut writer ).await
    {
        error!( "{:?}", e.kind() )
    }
}
