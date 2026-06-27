pub mod file;
pub mod grep;

pub trait Matcher {
    type Data<'a>
    where
        Self: 'a;

    type View<'a>
    where
        Self: 'a;

    fn open<'a>(&'a mut self, data: Self::Data<'a>);
    fn search(&mut self, filter: &str, append: bool);
    fn close(&mut self);
    fn tick(&mut self) -> Tick;
    fn slice<'a>(&'a self, offset: u32, limit: u32) -> Vec<Self::View<'a>>;
}

/// A matcher tick result, produced when a matcher is ticked
/// and it has yielded new results.
#[derive(Default)]
pub struct Tick {
    pub changed: bool,
    pub running: bool,
    pub matched: usize,
    pub total: usize,
}
