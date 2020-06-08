#[derive(Debug)]
pub struct Ref {
    pub id: String,
    pub name: String,
}

pub type Refs = Vec<Ref>;
