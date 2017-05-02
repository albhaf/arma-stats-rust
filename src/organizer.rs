use std::io;
use std::io::Read;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::thread::JoinHandle;

use hyper::Client;
use hyper::client::Response;
use serde_json;
use serde_json::Value;
use time;

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
            _worker: thread::spawn(move || for (path, data) in rx {
                                       match Organizer::send_event(&http, &path, &data) {
                                           Ok(_) => (),
                                           Err(e) => println!("{}", e),
                                       };
                                   }),
        }
    }

    pub fn call<'a>(&mut self, function: &'a str, data: &'a str) -> Option<&'a str> {
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
    fn panic(&self) -> Option<&'static str> {
        panic!("foobar");
    }

    fn setup(&mut self, data: &str) -> Option<&'static str> {
        self.hostname = Some(data.to_string());
        None
    }

    fn echo<'a>(&self, data: &'a str) -> Option<&'a str> {
        Some(data)
    }

    fn mission<'a>(&mut self, data: &'a str) -> Result<&'a str, &'a str> {
        try!(serde_json::from_str::<Value>(data).map_err(|_| "-1"));

        let path = match self.hostname {
            Some(ref s) => format!("{}/missions", s),
            None => return Err("-1"),
        };

        let mut res = try!(self.client.post(&path).body(data).send().map_err(|_| "-1"));

        let mut body = String::new();
        try!(res.read_to_string(&mut body).map_err(|_| "-1"));

        let mission: Value = try!(serde_json::from_str(&body).map_err(|_| "-1"));

        let mission_id: i64 = match mission.lookup("id") {
            Some(&Value::U64(id)) => id as i64,
            Some(&Value::String(ref id)) => try!(id.parse::<i64>().map_err(|_| "-1")),
            _ => return Err("-1"),
        };

        self.mission_id = mission_id;
        Ok("OK")
    }

    fn event<'a>(&self, data: &'a str) -> Result<&'a str, &'a str> {
        match serde_json::from_str::<Value>(data) {
            Ok(Value::Object(mut event)) => {
                event.insert("timestamp".to_string(),
                             Value::String(time::now().rfc3339().to_string()));
                let path = {
                    match self.hostname {
                        Some(ref s) => format!("{}/missions/{}/events", s, self.mission_id),
                        None => return Err("ERROR"),
                    }
                };
                let body = try!(serde_json::to_string(&event).map_err(|_| "ERROR"));
                try!(self.sender.as_ref().ok_or("ERROR"))
                    .send((path, body))
                    .unwrap();
                Ok("OK")
            }
            _ => Err("ERROR"),
        }
    }

    fn send_event(client: &Client, path: &str, data: &str) -> ::hyper::error::Result<Response> {
        match client.post(path).body(data).send() {
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

    use super::Organizer;

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
        assert_eq!(host, o.hostname.unwrap());
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
        assert_eq!(1, o.mission_id);

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
