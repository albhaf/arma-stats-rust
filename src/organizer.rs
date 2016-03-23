pub struct Organizer {
    hostname: Option<String>,
}

impl Organizer {
    pub fn new() -> Organizer {
        Organizer { hostname: None }
    }

    pub fn call<'a>(&mut self, function: &'a str, data: &'a str) -> Option<&'a str> {
        match function {
            "setup" => self.setup(data),
            "echo" => self.echo(data),
            _ => None,
        }
    }

    fn setup(&mut self, data: &str) -> Option<&'static str> {
        self.hostname = Some(data.to_string());
        None
    }

    fn echo<'a>(&self, data: &'a str) -> Option<&'a str> {
        Some(data)
    }
}
