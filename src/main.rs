use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod mtconnect;
use mtconnect::MTConnectAgent;

async fn handle_request(req: Request<Incoming>, agent: MTConnectAgent) -> Result<Response<String>, Infallible> {
    let path = req.uri().path();
    let method = req.method();

    match (method, path) {
        (&Method::GET, "/") => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html")
                .body("<h1>MTConnect Server</h1><p>Endpoints: /probe, /current, /sample</p>".to_string())
                .unwrap();
            Ok(response)
        }
        (&Method::GET, "/probe") => {
            let xml = agent.fetch_probe_from_all().await;
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/xml")
                .body(xml)
                .unwrap();
            Ok(response)
        }
        (&Method::GET, "/current") => {
            let xml = agent.fetch_current_from_all().await;
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/xml")
                .body(xml)
                .unwrap();
            Ok(response)
        }
        (&Method::GET, path) if path.starts_with("/sample") => {
            let xml = agent.fetch_current_from_all().await; // sample도 current와 동일
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/xml")
                .body(xml)
                .unwrap();
            Ok(response)
        }
        _ => {
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Not Found".to_string())
                .unwrap();
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let agent = MTConnectAgent::new();
    let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
    let listener = TcpListener::bind(addr).await?;

    println!("MTConnect server running on http://{}", addr);
    println!("Fetching data from remote agents: 192.168.20.10, 192.168.20.20, 192.168.20.30, 192.168.20.40, 192.168.20.50, 192.168.20.60, 192.168.20.70, 192.168.20.80, 192.168.20.100");

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let agent = agent.clone();

        tokio::task::spawn(async move {
            if let Err(err) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let agent = agent.clone();
                        handle_request(req, agent)
                    }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}