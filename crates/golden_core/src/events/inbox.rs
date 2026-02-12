use golden_schema::Event;

pub struct Inbox {
    pub events: Vec<Event>,
}

impl Inbox {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }
}
