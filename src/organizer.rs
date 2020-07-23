use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
};
use std::thread;
use std::thread::JoinHandle;

use reqwest::blocking::Client;
use serde_json;
use serde_json::Value;

pub struct Organizer {
    hostname: Option<String>,
    mission_id: i64,

    client: Arc<Client>,
    sender: Option<Sender<(String, String)>>,

    _worker: JoinHandle<()>,
}

impl Organizer {
    pub fn new() -> Organizer {
        let (tx, rx): (Sender<(String, String)>, Receiver<(String, String)>) = channel();
        let http = Arc::new(Client::new());

        Organizer {
            hostname: None,
            mission_id: 0,
            client: http.clone(),
            sender: Some(tx),
            _worker: thread::spawn(move || {
                for (path, data) in rx {
                    match Organizer::send_event(&http, &path, data) {
                        Ok(_) => (),
                        Err(e) => println!("{}", e),
                    };
                }
            }),
        }
    }

    pub fn call<'a>(&mut self, function: &'a str, data: String) -> Option<String> {
        match function {
            "setup" => self.setup(data),
            "echo" => self.echo(data),
            "mission" => Some(self.mission(data).unwrap_or_else(|e| e)),
            "event" => Some(self.event(data).unwrap_or_else(|e| e)),
            "panic" => self.panic(),
            _ => None,
        }
    }

    // Function only intended for testing panic handling and recovery
    fn panic(&self) -> Option<String> {
        panic!("foobar");
    }

    fn setup(&mut self, data: String) -> Option<String> {
        self.hostname = Some(data);
        None
    }

    fn echo<'a>(&self, data: String) -> Option<String> {
        Some(data)
    }

    fn mission(&mut self, data: String) -> Result<String, String> {
        (serde_json::from_str::<Value>(&data).map_err(|_| "-1"))?;

        let path = match self.hostname {
            Some(ref s) => format!("{}/missions", s),
            None => return Err("-1".to_string()),
        };

        let mut res = self
            .client
            .post(&path)
            .body(data)
            .send()
            .map_err(|_| "-1")?;

        let mut body = vec![];
        (res.copy_to(&mut body).map_err(|_| "-1"))?;
        let payload = String::from_utf8(body).map_err(|_| "-1")?;

        let mission: Value = (serde_json::from_str(&payload).map_err(|_| "-1"))?;

        let mission_id: i64 = mission
            .get("id")
            .expect("no id")
            .as_i64()
            .expect("invalid id");

        self.mission_id = mission_id;
        Ok("OK".to_string())
    }

    fn event<'a>(&self, data: String) -> Result<String, String> {
        match serde_json::from_str::<Value>(&data) {
            Ok(Value::Object(mut event)) => {
                event.insert(
                    "timestamp".to_string(),
                    Value::String(chrono::Utc::now().to_rfc3339().to_string()),
                );
                let path = {
                    match self.hostname {
                        Some(ref s) => format!("{}/missions/{}/events", s, self.mission_id),
                        None => return Err("ERROR".to_string()),
                    }
                };
                let body = (serde_json::to_string(&event).map_err(|_| "ERROR"))?;
                (self.sender.as_ref().ok_or("ERROR"))?
                    .send((path, body))
                    .unwrap();
                println!("queued up event");
                Ok("OK".to_string())
            }
            _ => Err("ERROR".to_string()),
        }
    }

    fn send_event(
        client: &Client,
        path: &str,
        data: String,
    ) -> ::reqwest::Result<::reqwest::blocking::Response> {
        println!("sending event");
        match client.post(path).body(data).send() {
            Ok(val) => Ok(val),
            // Err(::reqwest::Error::Http(e)) => {
            // Err(::reqwest::Error::Io(e)) => {
            // Retry in case of stale connection
            //     if e.kind() == io::ErrorKind::ConnectionAborted {
            //         client.post(path).body(data).send()
            //     } else {
            //         Err(::hyper::error::Error::Io(e))
            //     }
            // }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate iron;
    extern crate router;

    use super::Organizer;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use self::iron::prelude::*;
    use self::iron::status;
    use self::router::Router;

    #[test]
    fn setup() {
        let mut o = Organizer::new();
        let host = "http://localhost:8080";
        o.call("setup", host.to_string());
        assert_eq!(host, o.hostname.unwrap());
    }

    #[test]
    fn mission() {
        let mut router = Router::new();
        router.post(
            "/missions",
            |_r: &mut Request| Ok(Response::with((status::Ok, r#"{"id": 1}"#))),
            "missions",
        );

        let mut server = Iron::new(router).http("127.0.0.1:0").unwrap();

        let mut o = Organizer::new();
        o.call(
            "setup",
            "http://".to_string() + &(server.socket.to_string()),
        );
        let res = o
            .call("mission", r#"{"type": "empty"}"#.to_string())
            .unwrap();

        assert_eq!("OK", res);
        assert_eq!(1, o.mission_id);

        server.close().unwrap();
    }

    #[test]
    fn events() {
        // let mut router = Router::new();

        let event_counter = Arc::new(AtomicUsize::new(0));

        let server = {
            let event_counter = event_counter.clone();
            support::http(move |req| {
                let event_counter = event_counter.clone();
                async move {
                    match req.uri().path() {
                        "/missions" => http::Response::new(r#"{"id": 1}"#.into()),
                        "/missions/1/events" => {
                            println!("got event");
                            event_counter.fetch_add(1, Ordering::SeqCst);
                            http::Response::new("ok".into())
                        }
                        _ => panic!(),
                    }
                }
            })
        };

        let mut o = Organizer::new();
        o.call("setup", format!("http://{}", server.addr()));
        assert_eq!(
            Some("OK".to_string()),
            o.call("mission", r#"{"type": "empty"}"#.to_string())
        );
        assert_eq!(
            Some("OK".to_string()),
            o.call("event", r#"{"foo": "bar"}"#.to_string())
        );
        assert_eq!(
            Some("OK".to_string()),
            o.call("event", r#"{"foo": "bar"}"#.to_string())
        );

        o.sender = None;
        o._worker.join().unwrap();
        drop(server);

        assert_eq!(2, event_counter.load(Ordering::SeqCst));
    }

    /// Copied from https://github.com/seanmonstar/reqwest/blob/master/tests/support/server.rs
    /// TODO: extract somewhere
    mod support {
        use std::convert::Infallible;
        use std::future::Future;
        use std::net;
        use std::sync::mpsc as std_mpsc;
        use std::thread;
        use std::time::Duration;

        use tokio::sync::oneshot;

        pub use http::Response;
        use tokio::runtime;

        pub struct Server {
            addr: net::SocketAddr,
            panic_rx: std_mpsc::Receiver<()>,
            shutdown_tx: Option<oneshot::Sender<()>>,
        }

        impl Server {
            pub fn addr(&self) -> net::SocketAddr {
                self.addr
            }
        }

        impl Drop for Server {
            fn drop(&mut self) {
                if let Some(tx) = self.shutdown_tx.take() {
                    let _ = tx.send(());
                }

                if !::std::thread::panicking() {
                    self.panic_rx
                        .recv_timeout(Duration::from_secs(3))
                        .expect("test server should not panic");
                }
            }
        }

        pub fn http<F, Fut>(func: F) -> Server
        where
            F: Fn(http::Request<hyper::Body>) -> Fut + Clone + Send + 'static,
            Fut: Future<Output = http::Response<hyper::Body>> + Send + 'static,
        {
            //Spawn new runtime in thread to prevent reactor execution context conflict
            thread::spawn(move || {
                let mut rt = runtime::Builder::new()
                    .basic_scheduler()
                    .enable_all()
                    .build()
                    .expect("new rt");
                let srv = rt.block_on(async move {
                    hyper::Server::bind(&([127, 0, 0, 1], 0).into()).serve(
                        hyper::service::make_service_fn(move |_| {
                            let func = func.clone();
                            async move {
                                Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
                                    let fut = func(req);
                                    async move { Ok::<_, Infallible>(fut.await) }
                                }))
                            }
                        }),
                    )
                });

                let addr = srv.local_addr();
                let (shutdown_tx, shutdown_rx) = oneshot::channel();
                let srv = srv.with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                });

                let (panic_tx, panic_rx) = std_mpsc::channel();
                let tname = format!(
                    "test({})-support-server",
                    thread::current().name().unwrap_or("<unknown>")
                );
                thread::Builder::new()
                    .name(tname)
                    .spawn(move || {
                        rt.block_on(srv).unwrap();
                        let _ = panic_tx.send(());
                    })
                    .expect("thread spawn");

                Server {
                    addr,
                    panic_rx,
                    shutdown_tx: Some(shutdown_tx),
                }
            })
            .join()
            .unwrap()
        }
    }
}
