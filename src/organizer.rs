pub struct Organizer;

impl Organizer {
    pub const fn new() -> Organizer {
        Organizer
    }

    pub fn call<'a>(&self, function: &'a str, data: &'a str) -> Option<&'a str> {
        match function {
            "echo" => Some(data),
            _ => None,
        }
    }
}
