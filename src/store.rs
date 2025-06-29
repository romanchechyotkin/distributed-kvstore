use std::collections::HashMap;

pub struct Store {
    store: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Store {
        Store {
            store: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Option<String> {
        self.store.insert(key.to_string(), value.to_string())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).map(|s| s.as_str())
    }
}
