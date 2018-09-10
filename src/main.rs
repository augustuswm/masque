extern crate base64;
#[macro_use]
extern crate log;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;

use base64::encode;
use futures::future::{loop_fn, Loop};
use hyper::header::{HeaderValue, ACCEPT, AUTHORIZATION};
use hyper::rt::{self, Future, Stream};
use hyper::service::service_fn_ok;
use hyper::{Body, Client, Method, Request, Response, Server};
use hyper_tls::HttpsConnector;

use event::{Event, EventBuilder, EventLineResponse};
use store::Store;

mod error;
mod event;
mod store;

#[derive(Debug)]
enum EventStreamResponse {
    Complete(Event),
    Incomplete,
}

#[derive(Debug)]
struct StreamHandler {
    b: EventBuilder,
}

impl StreamHandler {
    pub fn new() -> StreamHandler {
        StreamHandler {
            b: EventBuilder::new(),
        }
    }

    pub fn read<T>(&mut self, lines: Vec<T>) -> EventStreamResponse
    where
        T: AsRef<str>,
    {
        match self.b.clone().read_in_lines(lines) {
            EventLineResponse::Continue(b) => {
                self.b = b;
                EventStreamResponse::Incomplete
            }
            EventLineResponse::Complete(event) => {
                self.b = EventBuilder::new();
                EventStreamResponse::Complete(event)
            }
        }
    }
}

struct StreamReq<'a, 'b, 'c, 'd, 'e> {
    pub app: &'a str,
    pub env: &'b str,
    pub key: &'c str,
    pub secret: &'d str,
    pub host: Option<&'e str>,
    pub port: Option<u16>,
}

impl<'a, 'b, 'c, 'd, 'e> StreamReq<'a, 'b, 'c, 'd, 'e> {
    pub fn as_request(&self) -> Result<Request<Body>, error::MasqueError> {
        let url = "https://".to_string() + self.host.unwrap_or("localhost") + ":"
            + self.port.unwrap_or(8088).to_string().as_str() + "/api/v1/stream/"
            + self.app + "/" + self.env + "/";

        let auth = "Basic ".to_string() + &encode(&("".to_string() + self.key + ":" + self.secret));

        let mut r = Request::new(Body::from(""));
        *r.method_mut() = Method::GET;
        *r.uri_mut() = url.parse().map_err(|_| error::MasqueError::InvalidUri)?;
        r.headers_mut()
            .insert(ACCEPT, HeaderValue::from_str("text/event-stream")?);
        r.headers_mut()
            .insert(AUTHORIZATION, HeaderValue::from_str(&auth)?);

        Ok(r)
    }
}

fn request(store: Store<String>) -> impl Future<Item = Loop<(), Store<String>>, Error = ()> {
    let r = StreamReq {
        app: "test_app",
        env: "test_env",
        key: "dev",
        secret: "dev",
        host: Some("www.masquerade.io"),
        port: Some(443),
    };

    let mut h = StreamHandler::new();

    let https = HttpsConnector::new(4).expect("TLS initialization failed");
    let client = Client::builder().build::<_, hyper::Body>(https);

    let s_ok = store.clone();
    let s_err = store.clone();

    client
        .request(r.as_request().unwrap())
        .and_then(move |res| {
            println!("Response: {}", res.status());
            println!("Headers: {:#?}", res.headers());
            res.into_body().for_each(move |chunk| {
                let _written = ::std::str::from_utf8(&*chunk)
                    .map_err(|e| e.into())
                    .and_then(|data| {
                        let lines = data.split("\n").collect::<Vec<&str>>();

                        match h.read(lines) {
                            EventStreamResponse::Incomplete => Ok(()),
                            EventStreamResponse::Complete(event) => {
                                println!("Stored {:?}", event);
                                store.update(event.data()).and_then(|_| Ok(()))
                            }
                        }
                    })
                    .or_else(|e| {
                        error!("Error handling incoming data: {}", e);
                        Err(e)
                    });

                Ok(())
            })
        })
        .and_then(|_| {
            Ok(Loop::Continue(s_ok))
        })
        .or_else(|_err| {
            Ok(Loop::Continue(s_err))
        })
}

fn main() -> Result<(), error::MasqueError> {
    let s: Store<String> = Store::new("");

    let proxy = loop_fn(s.clone(), request);

    let new_service = move || {
        let s_r = s.clone();
        service_fn_ok(move |_| {
            let data = s_r.get().unwrap();
            println!("Sending {:?}", data);
            Response::new(Body::from(data))
        })
    };

    let addr = ([127, 0, 0, 1], 3459).into();

    let server = Server::bind(&addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));

    rt::run(rt::lazy(move || {
        rt::spawn(proxy);
        server
    }));

    Ok(())
}
