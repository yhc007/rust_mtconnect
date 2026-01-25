use hyper::service::service_fn;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

mod mtconnect;
use mtconnect::MTConnectAgent;
use elfin_postgres_data_access::DatabaseService;

/// 장비 이름 반환 (device.ip에 이미 장비 이름이 저장됨)
fn get_machine_name(device_name: &str) -> String {
    device_name.to_string()
}

async fn handle_request(
    req: Request<Incoming>,
    agent: MTConnectAgent,
    db: Option<Arc<DatabaseService>>,
) -> Result<Response<String>, Infallible> {
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
            let (xml, devices) = agent.fetch_current_with_devices().await;

            // DB에 데이터 저장
            if let Some(db) = &db {
                for device in devices.iter() {
                    let machine_id = get_machine_name(&device.ip);
                    let cnc_data = device.to_cnc_data(&machine_id, 1);

                    // 실시간 상태 업데이트
                    if let Err(e) = db.update_machine_status(&cnc_data).await {
                        eprintln!("Failed to update machine status for {}: {:?}", machine_id, e);
                    }

                    // 이력 저장
                    if let Err(e) = db.save_machine_data_history(&cnc_data).await {
                        eprintln!("Failed to save history for {}: {:?}", machine_id, e);
                    }
                }
            }

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
    // 환경 변수 로드
    dotenv::dotenv().ok();

    let agent = MTConnectAgent::new();
    let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
    let listener = TcpListener::bind(addr).await?;

    // 데이터베이스 연결 시도
    let db: Option<Arc<DatabaseService>> = match DatabaseService::from_env().await {
        Ok(service) => {
            println!("Database connected successfully");
            if let Err(e) = service.test_connection().await {
                eprintln!("Database connection test failed: {:?}", e);
                None
            } else {
                Some(Arc::new(service))
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to database: {:?}", e);
            eprintln!("Server will run without database storage");
            None
        }
    };

    println!("MTConnect server running on http://{}", addr);
    println!("Fetching data from remote agents:");
    println!("  - HCN6800 (http://192.168.10.5:5000)");
    println!("  - QT350-1 (http://192.168.10.5:5001)");
    println!("  - QT350-2 (http://192.168.10.5:5002)");
    println!("  - QT350-3 (http://192.168.10.5:5003)");
    println!("  - QT350-4 (http://192.168.10.5:5004)");
    println!("  - MNT600-1 (http://192.168.10.5:5005)");
    println!("  - MNT600S-1 (http://192.168.10.5:5006)");
    println!("  - MNT600-2 (http://192.168.10.5:5007)");
    println!("  - MNT600S-2 (http://192.168.10.5:5008)");
    if db.is_some() {
        println!("Database storage: ENABLED");
    } else {
        println!("Database storage: DISABLED");
    }

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let agent = agent.clone();
        let db = db.clone();

        tokio::task::spawn(async move {
            if let Err(err) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let agent = agent.clone();
                        let db = db.clone();
                        handle_request(req, agent, db)
                    }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}