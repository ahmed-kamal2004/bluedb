pub trait HasDestructor {}

pub trait ListState: HasDestructor {
    type Item;
    type RawItem;

    fn new() -> Self;
    fn push_front(&mut self, item: Self::Item);
    fn push_back(&mut self, item: Self::Item);
    fn pop_front(&mut self) -> Option<Self::Item>;
    fn pop_back(&mut self) -> Option<Self::Item>;
    fn head(&self) -> Self::RawItem;
    fn next(&self, curr: &mut Self::RawItem) -> Option<Self::Item>;
}
