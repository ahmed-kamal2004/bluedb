use super::super::pool::page::Page;
pub struct BtreePageView<'a> {
    pub page: &'a mut Page,
}

impl<'a> BtreePageView<'a> {
    pub fn new(page: &'a mut Page) -> Self {
        BtreePageView { page }
    }
}
