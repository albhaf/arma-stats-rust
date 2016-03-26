use std::io;
use std::io::Read;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::RwLock;
use std::thread;
use std::thread::JoinHandle;

use hyper::Client;
use hyper::client::Response;
use serde_json;
use serde_json::Value;
use time;

pub struct Organizer {
    settings: Arc<Settings>,

    client: Arc<Client>,
    sender: Option<Sender<String>>,

    _worker: JoinHandle<()>,
}

struct Settings {
    hostname: RwLock<Option<String>>,
    mission_id: RwLock<i64>,
}


impl Organizer {
    pub fn new() -> Organizer {
        let (tx, rx): (Sender<String>, Receiver<String>) = channel();

        let http = Arc::new(Client::new());
        let settings = Arc::new(Settings {
            hostname: RwLock::new(None),
            mission_id: RwLock::new(0),
        });

        let worker = {
            let client = http.clone();
            let settings = settings.clone();
            thread::spawn(move || {
                for data in rx {
                    let path = {
                        let hostname = settings.hostname.read().unwrap();
                        let mission = settings.mission_id.read().unwrap();
                        match *hostname {
                            Some(ref s) => {
                                format!("{}/missions/{}/events", s.to_string(), *mission)
                            }
                            None => continue,
                        }
                    };

                    match Organizer::send_event(&client, &path, &data) {
                        Ok(_) => (),
                        Err(e) => println!("{}", e),
                    };
                }
            })
        };

        Organizer {
            settings: settings.clone(),
            client: http.clone(),
            sender: Some(tx),
            _worker: worker,
        }
    }

    pub fn call<'a>(&mut self, function: &'a str, data: &'a str) -> Option<&'a str> {
        match function {
            "setup" => self.setup(data),
            "echo" => self.echo(data),
            "mission" => self.mission(data),
            "event" => self.event(data),
            _ => None,
        }
    }

    fn setup(&mut self, data: &str) -> Option<&'static str> {
        let mut guard = self.settings.hostname.write().unwrap();
        *guard = Some(data.to_string());
        None
    }

    fn echo<'a>(&self, data: &'a str) -> Option<&'a str> {
        Some(data)
    }

    fn mission<'a>(&mut self, data: &'a str) -> Option<&'a str> {
        match serde_json::from_str::<Value>(data) {
            Err(_) => return Some("-1"),
            _ => (),
        };

        let path = {
            let guard = self.settings.hostname.read().unwrap();
            match *guard {
                Some(ref s) => format!("{}/missions", s.to_string()),
                None => return Some("-1"),
            }
        };
        let mut res = self.client
                          .post(&path)
                          .body(data)
                          .send()
                          .unwrap(); //TODO: handle error

        let mut body = String::new();
        res.read_to_string(&mut body).unwrap(); //TODO: error

        let mission: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => return Some("-1"),
        };

        let mission_id: i64 = match mission.lookup("id") {
            Some(&Value::U64(id)) => id as i64,
            Some(&Value::String(ref id)) => {
                match id.parse::<i64>() {
                    Ok(n) => n,
                    Err(_) => return Some("-1"),
                }
            }
            _ => return Some("-1"),
        };

        let mut guard = self.settings.mission_id.write().unwrap();
        *guard = mission_id;
        Some("OK")
    }

    fn event<'a>(&self, data: &'a str) -> Option<&'a str> {
        let event = match serde_json::from_str::<Value>(data) {
            Ok(mut v) => {
                match v.as_object_mut() {
                    Some(e) => {
                        e.insert("timestamp".to_string(),
                                 Value::String(time::now().rfc3339().to_string()));
                    }
                    None => return Some("ERROR"),
                }
            }
            Err(_) => return Some("ERROR"),
        };

        let body = match serde_json::to_string(&event) {
            Ok(v) => v,
            Err(_) => return Some("ERROR"),
        };

        match self.sender {
            Some(ref s) => s.send(body).unwrap(),
            None => (),
        };
        Some("OK")
    }

    fn send_event(client: &Client, path: &str, data: &str) -> ::hyper::error::Result<Response> {
        match client.post(path)
                    .body(data)
                    .send() {
            Ok(val) => Ok(val),
            Err(::hyper::error::Error::Io(e)) => {
                // Retry in case of stale connection
                if e.kind() == io::ErrorKind::ConnectionAborted {
                    client.post(path).body(data).send()
                } else {
                    Err(::hyper::error::Error::Io(e))
                }
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate iron;
    extern crate router;

    use super::*;

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use self::iron::prelude::*;
    use self::iron::status;
    use self::router::Router;

    #[test]
    fn setup() {
        let mut o = Organizer::new();
        let host = "http://localhost:8080";
        o.call("setup", host);
        assert_eq!(host,
                   *(o.settings.hostname.read().unwrap()).as_ref().unwrap());
    }

    #[test]
    fn mission() {
        let mut router = Router::new();
        router.post("/missions",
                    |_r: &mut Request| Ok(Response::with((status::Ok, r#"{"id": 1}"#))));

        let mut server = Iron::new(router).http("127.0.0.1:0").unwrap();

        let mut o = Organizer::new();
        o.call("setup",
               &("http://".to_string() + &(server.socket.to_string())));
        let res = o.call("mission", r#"{"type": "empty"}"#).unwrap();

        assert_eq!("OK", res);
        assert_eq!(1, *(o.settings.mission_id.read().unwrap()));

        server.close().unwrap();
    }

    #[test]
    fn events() {
        let mut router = Router::new();

        let event_counter = Arc::new(AtomicUsize::new(0));

        {
            let event_counter = event_counter.clone();
            router.post("/missions",
                        |_r: &mut Request| Ok(Response::with((status::Ok, r#"{"id": 1}"#))));
            router.post("/missions/1/events", move |_r: &mut Request| {
                event_counter.fetch_add(1, Ordering::Relaxed);
                Ok(Response::with((status::Ok, "ok")))
            });
        }

        let mut server = Iron::new(router).http("127.0.0.1:0").unwrap();

        let mut o = Organizer::new();
        o.call("setup",
               &("http://".to_string() + &(server.socket.to_string())));
        o.call("mission", r#"{"type": "empty"}"#);
        o.call("event", r#"{"foo": "bar"}"#);
        o.call("event", r#"{"foo": "bar"}"#);

        o.sender = None;
        o._worker.join().unwrap();
        server.close().unwrap();

        assert_eq!(2, event_counter.load(Ordering::Relaxed));
    }
}
