#[macro_use]
extern crate rocket;

#[cfg(test)]
mod tests;

use rocket::form::{Form, FromForm};
use rocket::fs::{relative, FileServer};
use rocket::response::stream::{Event, EventStream};
use rocket::Shutdown;
use rocket::{
    serde::{Deserialize, Serialize},
    tokio::select,
    tokio::sync::broadcast::{channel, error::RecvError, Sender},
    State,
};

// https://www.youtube.com/watch?v=NS9Dh63i_Q4&t=2s

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate = "rocket::serde")]
struct Message {
    #[field(validate = len(..30))]
    pub room: String,
    #[field(validate = len(..20))]
    pub username: String,
    pub message: String,
}

// endpoint to send messages
#[post("/message", data = "<form>")]
fn post(form: Form<Message>, queue: &State<Sender<Message>>) {
    // A send 'fails' if there are no active subscribers. That's ok
    let _res = queue.send(form.into_inner());
}

// endpoint to receive messages
// The Server can send the data to client but the client can't send data to server

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    // create new receiver
    let mut rx = queue.subscribe();

    EventStream! {
        loop {
            // select macro = waits on multiple concurent braches(futures) at the same time
            // if one of the branches is ready, it will be executed
            // if none of the branches is ready, it will wait until one of them is ready

            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
            _ = &mut end=> break,

            };
            // SSE is a stream of events, so we need to yield each event
            yield Event::json(&msg);
        }
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .mount("/", routes![post, events])
        // a handler to serve static files
        .mount("/", FileServer::from(relative!("static")))
}
