#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

// Define type aliases for convenience
type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// Define a struct for the 'Event'
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Event {
    id: u64,
    name: String,
    description: String,
    date: String,
    start_time: String,
    location: String,
    attendee_ids: Vec<u64>,
    ticket_ids: Vec<u64>,
    created_at: u64,
    updated_at: Option<u64>,
}

// Define a struct for the 'User'
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct User {
    id: u64,
    name: String,
    email: String,
    password: String,
    event_ids: Vec<u64>,
    ticket_ids: Vec<u64>,
    created_at: u64,
    updated_at: Option<u64>,
}

// Define a struct for the 'Ticket'
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Ticket {
    id: u64,
    event_id: u64,
    user_id: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

// Implement the 'Storable' trait for 'Event', 'User', and 'Ticket'
impl Storable for Event {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl Storable for User {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl Storable for Ticket {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// Implement the 'BoundedStorable' trait for 'Event', 'User', and 'Ticket'
impl BoundedStorable for Event {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl BoundedStorable for User {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl BoundedStorable for Ticket {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Define thread-local static variables for memory management and storage
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static EVENT_STORAGE: RefCell<StableBTreeMap<u64, Event, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static USER_STORAGE: RefCell<StableBTreeMap<u64, User, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static TICKET_STORAGE: RefCell<StableBTreeMap<u64, Ticket, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));
}

// Define structs for payload data (used in update calls)
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct EventPayload {
    name: String,
    description: String,
    date: String,
    start_time: String,
    location: String,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct UserPayload {
    name: String,
    email: String,
    password: String,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TicketPayload {
    event_id: u64,
    user_id: u64,
}

// Define the Candid interface
#[ic_cdk::query]
fn get_all_events() -> Vec<Event> {
    EVENT_STORAGE.with(|events| {
        events
            .borrow()
            .iter()
            .map(|(_, event)| event.clone())
            .collect()
    })
}

#[ic_cdk::query]
fn get_event(id: u64) -> Result<Event, Error> {
    match _get_event(&id) {
        Some(event) => Ok(event),
        None => Err(Error::NotFound {
            msg: format!("event id:{} does not exist", id),
        }),
    }
}

fn _get_event(id: &u64) -> Option<Event> {
    EVENT_STORAGE.with(|events| events.borrow().get(id).cloned())
}

#[ic_cdk::update]
fn create_event(payload: EventPayload) -> Result<Event, Error> {
    let id = increment_id_counter()?;

    let event = Event {
        id,
        name: payload.name,
        description: payload.description,
        date: payload.date,
        start_time: payload.start_time,
        location: payload.location,
        attendee_ids: vec![],
        ticket_ids: vec![],
        created_at: time(),
        updated_at: None,
    };

    EVENT_STORAGE.with(|events| events.borrow_mut().insert(id, event.clone()));

    Ok(event)
}

#[ic_cdk::update]
fn update_event(id: u64, payload: EventPayload) -> Result<Event, Error> {
    let mut event = _get_event(&id).ok_or(Error::NotFound {
        msg: format!("event id:{} does not exist", id),
    })?;

    event.name = payload.name;
    event.description = payload.description;
    event.date = payload.date;
    event.start_time = payload.start_time;
    event.location = payload.location;
    event.updated_at = Some(time());

    EVENT_STORAGE.with(|events| events.borrow_mut().insert(id, event.clone()));

    Ok(event)
}

#[ic_cdk::update]
fn delete_event(id: u64) -> Result<String, Error> {
    if _get_event(&id).is_none() {
        return Err(Error::NotFound {
            msg: format!("event id:{} does not exist", id),
        });
    }

    EVENT_STORAGE.with(|events| events.borrow_mut().remove(&id));

    Ok(format!("event id: {} deleted", id))
}

#[ic_cdk::query]
fn get_user(id: u64) -> Result<User, Error> {
    match _get_user(&id) {
        Some(user) => Ok(user),
        None => Err(Error::NotFound {
            msg: format!("user id:{} does not exist", id),
        }),
    }
}

fn _get_user(id: &u64) -> Option<User> {
    USER_STORAGE.with(|users| users.borrow().get(id).cloned())
}

#[ic_cdk::update]
fn create_user(payload: UserPayload) -> Result<User, Error> {
    let id = increment_id_counter()?;

    let user = User {
        id,
        name: payload.name,
        email: payload.email,
        password: payload.password,
        event_ids: vec![],
        ticket_ids: vec![],
        created_at: time(),
        updated_at: None,
    };

    USER_STORAGE.with(|users| users.borrow_mut().insert(id, user.clone()));

    Ok(user)
}

#[ic_cdk::update]
fn update_user(id: u64, payload: UserPayload) -> Result<User, Error> {
    let mut user = _get_user(&id).ok_or(Error::NotFound {
        msg: format!("user id:{} does not exist", id),
    })?;

    user.name = payload.name;
    user.email = payload.email;
    user.password = payload.password;
    user.updated_at = Some(time());

    USER_STORAGE.with(|users| users.borrow_mut().insert(id, user.clone()));

    Ok(user)
}

#[ic_cdk::update]
fn delete_user(id: u64) -> Result<String, Error> {
    if _get_user(&id).is_none() {
        return Err(Error::NotFound {
            msg: format!("user id:{} does not exist", id),
        });
    }

    USER_STORAGE.with(|users| users.borrow_mut().remove(&id));

    Ok(format!("user id: {} deleted", id))
}

#[ic_cdk::query]
fn get_ticket(id: u64) -> Result<Ticket, Error> {
    match _get_ticket(&id) {
        Some(ticket) => Ok(ticket),
        None => Err(Error::NotFound {
            msg: format!("ticket id:{} does not exist", id),
        }),
    }
}

fn _get_ticket(id: &u64) -> Option<Ticket> {
    TICKET_STORAGE.with(|tickets| tickets.borrow().get(id).cloned())
}

#[ic_cdk::update]
fn create_ticket(payload: TicketPayload) -> Result<Ticket, AssociationError> {
    let id = increment_id_counter().map_err(|e| AssociationError::Err {
        msg: format!("Failed to increment ID counter: {}", e.msg),
        ticket: Ticket::default(),
    })?;

    let ticket = Ticket {
        id,
        event_id: payload.event_id,
        user_id: payload.user_id,
        created_at: time(),
        updated_at: None,
    };

    TICKET_STORAGE.with(|tickets| tickets.borrow_mut().insert(id, ticket.clone()));

    if let Err(err) = add_event_attendee(payload.event_id, payload.user_id) {
        return Err(AssociationError::Err {
            msg: format!("Could not add attendee to event id:{} ", payload.event_id),
            ticket: ticket.clone(),
        });
    }

    if let Err(err) = add_user_ticket(payload.user_id, id) {
        return Err(AssociationError::Err {
            msg: format!("Could not add ticket id:{} to user id:{} ", id, payload.user_id),
            ticket: ticket.clone(),
        });
    }

    if let Err(err) = add_event_ticket(payload.event_id, id) {
        return Err(AssociationError::Err {
            msg: format!("Could not add ticket id:{} to event id:{} ", id, payload.event_id),
            ticket: ticket.clone(),
        });
    }

    Ok(ticket)
}

#[ic_cdk::update]
fn update_ticket(id: u64, payload: TicketPayload) -> Result<Ticket, Error> {
    let mut ticket = _get_ticket(&id).ok_or(Error::NotFound {
        msg: format!("ticket id:{} does not exist", id),
    })?;

    if payload.user_id != ticket.user_id {
        remove_user_ticket(ticket.user_id, ticket.id)?;
        add_user_ticket(payload.user_id, ticket.id)?;
        ticket.user_id = payload.user_id;
    }

    if payload.event_id != ticket.event_id {
        remove_event_ticket(ticket.event_id, ticket.id)?;
        add_event_ticket(payload.event_id, ticket.id)?;
        ticket.event_id = payload.event_id;
    }

    ticket.updated_at = Some(time());

    TICKET_STORAGE.with(|tickets| tickets.borrow_mut().insert(id, ticket.clone()));

    Ok(ticket)
}

#[ic_cdk::update]
fn delete_ticket(id: u64) -> Result<String, Error> {
    let ticket = _get_ticket(&id).ok_or(Error::NotFound {
        msg: format!("ticket id:{} does not exist", id),
    })?;

    remove_user_ticket(ticket.user_id, ticket.id)?;
    remove_event_ticket(ticket.event_id, ticket.id)?;

    TICKET_STORAGE.with(|tickets| tickets.borrow_mut().remove(&id));

    Ok(format!("ticket id: {} deleted", id))
}

#[ic_cdk::query]
fn get_event_attendees(id: u64) -> Result<Vec<User>, Error> {
    let event = _get_event(&id).ok_or(Error::NotFound {
        msg: format!("event id:{} does not exist", id),
    })?;

    let attendees: Result<Vec<User>, Error> = event.attendee_ids.iter().map(|&attendee_id| {
        _get_user(&attendee_id).ok_or(Error::NotFound {
            msg: format!("user id:{} does not exist", attendee_id),
        })
    }).collect();

    attendees
}

fn add_event_attendee(event_id: u64, user_id: u64) -> Result<(), Error> {
    let mut event = _get_event(&event_id).ok_or(Error::NotFound {
        msg: format!("event id:{} does not exist", event_id),
    })?;

    if !event.attendee_ids.contains(&user_id) {
        event.attendee_ids.push(user_id);
        event.updated_at = Some(time());
        EVENT_STORAGE.with(|events| events.borrow_mut().insert(event_id, event));
    }

    Ok(())
}

fn add_event_ticket(event_id: u64, ticket_id: u64) -> Result<(), Error> {
    let mut event = _get_event(&event_id).ok_or(Error::NotFound {
        msg: format!("event id:{} does not exist", event_id),
    })?;

    if !event.ticket_ids.contains(&ticket_id) {
        event.ticket_ids.push(ticket_id);
        event.updated_at = Some(time());
        EVENT_STORAGE.with(|events| events.borrow_mut().insert(event_id, event));
    }

    Ok(())
}

#[ic_cdk::query]
fn get_user_tickets(id: u64) -> Result<Vec<Ticket>, Error> {
    let user = _get_user(&id).ok_or(Error::NotFound {
        msg: format!("user id:{} does not exist", id),
    })?;

    let tickets: Result<Vec<Ticket>, Error> = user.ticket_ids.iter().map(|&ticket_id| {
        _get_ticket(&ticket_id).ok_or(Error::NotFound {
            msg: format!("ticket id:{} does not exist", ticket_id),
        })
    }).collect();

    tickets
}

#[ic_cdk::query]
fn get_event_tickets(id: u64) -> Result<Vec<Ticket>, Error> {
    let event = _get_event(&id).ok_or(Error::NotFound {
        msg: format!("event id:{} does not exist", id),
    })?;

    let tickets: Result<Vec<Ticket>, Error> = event.ticket_ids.iter().map(|&ticket_id| {
        _get_ticket(&ticket_id).ok_or(Error::NotFound {
            msg: format!("ticket id:{} does not exist", ticket_id),
        })
    }).collect();

    tickets
}

fn add_user_ticket(user_id: u64, ticket_id: u64) -> Result<(), Error> {
    let mut user = _get_user(&user_id).ok_or(Error::NotFound {
        msg: format!("user id:{} does not exist", user_id),
    })?;

    if !user.ticket_ids.contains(&ticket_id) {
        user.ticket_ids.push(ticket_id);
        user.updated_at = Some(time());
        USER_STORAGE.with(|users| users.borrow_mut().insert(user_id, user));
    }

    Ok(())
}

fn remove_user_ticket(user_id: u64, ticket_id: u64) -> Result<(), Error> {
    let mut user = _get_user(&user_id).ok_or(Error::NotFound {
        msg: format!("user id:{} does not exist", user_id),
    })?;

    user.ticket_ids.retain(|&id| id != ticket_id);
    user.updated_at = Some(time());
    USER_STORAGE.with(|users| users.borrow_mut().insert(user_id, user));

    Ok(())
}

fn remove_event_ticket(event_id: u64, ticket_id: u64) -> Result<(), Error> {
    let mut event = _get_event(&event_id).ok_or(Error::NotFound {
        msg: format!("event id:{} does not exist", event_id),
    })?;

    event.ticket_ids.retain(|&id| id != ticket_id);
    event.updated_at = Some(time());
    EVENT_STORAGE.with(|events| events.borrow_mut().insert(event_id, event));

    Ok(())
}

fn increment_id_counter() -> Result<u64, Error> {
    ID_COUNTER.with(|counter| {
        let current_id = *counter.borrow().get();
        counter.borrow_mut().set(current_id + 1).map_err(|_| Error::NotCreated {
            msg: "Failed to increment ID counter".to_string()
        })?;
        Ok(current_id + 1)
    })
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    NotCreated { msg: String },
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum AssociationError {
    Err { msg: String, ticket: Ticket },
}

ic_cdk::export_candid!();
