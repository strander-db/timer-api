use std::convert::Infallible;
use std::fs::File;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rand::Rng;
use tokio::net::TcpListener;

const ARBITRARY_DURATION: Duration = Duration::from_secs(1722489600);

fn start_time() -> SystemTime {
    UNIX_EPOCH + ARBITRARY_DURATION
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 7654));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on port {}", addr.port());
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let rng = rand::rng().random_range(100..1000);
        let rng = std::sync::Arc::new(tokio::sync::Mutex::new(rng));
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handler(req, rng.clone())))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn handler(
    request: Request<hyper::body::Incoming>,
    rng: std::sync::Arc<tokio::sync::Mutex<u64>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let result;
    if request.method() == hyper::Method::OPTIONS {
        let response = Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
            .body(Full::new(Bytes::from("")))
            .unwrap();
        result = Ok(response);
    } else if request.uri().path().starts_with("/reset/") && request.method() == hyper::Method::POST
    {
        result = reset(request).await;
    } else if request.uri().path() == "/list" && request.method() == hyper::Method::GET {
        result = list().await;
    } else if request.method() == hyper::Method::GET {
        result = timestamp(request).await;
    } else {
        result = Ok(Response::builder()
            .status(404)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap());
    }
    tokio::time::sleep(Duration::from_millis(*rng.lock().await)).await;
    result
}

async fn reset(
    request: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let name = request.uri().path().trim_start_matches("/reset/");
    if name == "list" {
        return Ok(Response::builder()
            .status(403)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
            .body(Full::new(Bytes::from("Can't name timer \"list\"")))
            .unwrap());
    }
    let path = Path::new("./").join(name);
    let mut file = File::create(path).unwrap();
    let timestamp = SystemTime::now().duration_since(start_time()).unwrap();
    file.write_all(timestamp.as_secs().to_string().as_bytes())
        .unwrap();
    let mut list = String::new();
    match File::open(Path::new("./").join("list")) {
        Ok(mut file) => {
            file.read_to_string(&mut list).unwrap();
        }
        Err(_) => {
            list = String::from("[]");
        }
    };
    let mut list = list
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .collect::<Vec<&str>>();
    if !list.contains(&name) {
        list.push(name);
    }
    let list_string = format!(
        "[{}]",
        list.iter()
            .filter(|x| !x.is_empty())
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(",")
    );
    File::create(Path::new("./").join("list"))
        .unwrap()
        .write_all(list_string.as_bytes())
        .unwrap();
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        .body(Full::new(Bytes::from(format!(
            "{{ \"start\": {}, \"elapsed\": 0 }}",
            timestamp.as_secs()
        ))))
        .unwrap();
    Ok(response)
}

async fn list() -> Result<Response<Full<Bytes>>, Infallible> {
    let mut list_string = String::new();
    match File::open(Path::new("./").join("list")) {
        Ok(mut file) => {
            file.read_to_string(&mut list_string).unwrap();
        }
        Err(_) => {
            list_string = String::from("[]");
        }
    };
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        .body(Full::new(Bytes::from(list_string)))
        .unwrap();
    Ok(response)
}

async fn timestamp(
    request: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = request.uri().path();
    let path = Path::new("./").join(path.trim_start_matches('/'));
    match File::open(path) {
        Ok(mut file) => {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).unwrap();
            let timestamp = buffer.parse::<u64>().unwrap();
            let elapsed = SystemTime::now()
                .duration_since(start_time())
                .unwrap()
                .as_secs()
                - timestamp;
            let response = Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Headers", "*")
                .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
                .body(Full::new(Bytes::from(format!(
                    "{{ \"start\": {}, \"elapsed\": {} }}",
                    timestamp, elapsed
                ))))
                .unwrap();
            Ok(response)
        }
        Err(_) => {
            let response = Response::builder()
                .status(404)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Headers", "*")
                .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap();
            Ok(response)
        }
    }
}
