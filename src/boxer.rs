#[derive(Debug)]
struct Boxer {
    id: u32,
    name: String,
    boxrec_score: f32,
}

impl Boxer {
    pub fn get_by_name(name: &String) -> Option<Boxer> {
        // TODO
        None
    }

    pub fn get_by_id(id: &u32) -> Option<Boxer> {
        // TODO
        None
    }

    pub fn difference_vs(&self, opponent: &Boxer) -> f32 {
        self.boxrec_score - opponent.boxrec_score
    }
}